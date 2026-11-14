#include "app.h"

#include <malloc.h>

#include <QtCore/QDirIterator>
#include <QtCore/QFile>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtGui/QColor>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmapCache>
#include <QtGui/QWindow>
#include <QtQml/QQmlApplicationEngine>
#include <QtQuick/QQuickImageProvider>
#include <QtSvg/QSvgRenderer>

namespace {

// Size used when QML asks for no particular size.
constexpr int defaultSide = 16;

QSize sizeFor(const QSize& requested)
{
    return QSize(requested.width() > 0 ? requested.width() : defaultSide,
                 requested.height() > 0 ? requested.height() : defaultSide);
}

// Lucide icons are drawn with `currentColor`; the colour asked for replaces it.
class LucideProvider : public QQuickImageProvider {
public:
    LucideProvider()
        : QQuickImageProvider(QQuickImageProvider::Image)
    {
    }

    QImage requestImage(const QString& id, QSize* size, const QSize& requested) override
    {
        const qsizetype slash = id.indexOf(QLatin1Char('/'));
        const QString name = slash < 0 ? id : id.left(slash);
        const QColor colour(QLatin1Char('#') + (slash < 0 ? QStringLiteral("ffffffff") : id.mid(slash + 1)));
        QImage image(sizeFor(requested), QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::transparent);
        QFile file(QStringLiteral(":/qt/qml/Rustphin/icons/%1.svg").arg(name));
        if (file.open(QIODevice::ReadOnly)) {
            QByteArray svg = file.readAll();
            svg.replace("currentColor", colour.name(QColor::HexRgb).toUtf8());
            QSvgRenderer renderer(svg);
            QPainter painter(&image);
            painter.setOpacity(colour.alphaF());
            renderer.render(&painter);
        }
        if (size != nullptr) {
            *size = image.size();
        }
        return image;
    }
};

// File-type icons from the desktop's icon theme, most specific name first.
class ThemeProvider : public QQuickImageProvider {
public:
    ThemeProvider()
        : QQuickImageProvider(QQuickImageProvider::Pixmap)
    {
    }

    QPixmap requestPixmap(const QString& id, QSize* size, const QSize& requested) override
    {
        const QSize side = sizeFor(requested);
        QPixmap pixmap;
        // QML hands the id over percent-encoded ("|" arrives as "%7C").
        const QString names = QUrl::fromPercentEncoding(id.toUtf8());
        for (const QString& name : names.split(QLatin1Char('|'), Qt::SkipEmptyParts)) {
            const QIcon icon = QIcon::fromTheme(name);
            if (!icon.isNull()) {
                pixmap = icon.pixmap(side);
                break;
            }
        }
        if (pixmap.isNull()) {
            pixmap = QPixmap(side);
            pixmap.fill(Qt::transparent);
        }
        if (size != nullptr) {
            *size = pixmap.size();
        }
        return pixmap;
    }
};

struct Shell {
    QQmlApplicationEngine* engine = nullptr;
    bool keepRunning = true;
    QString home;
};

Shell& shell()
{
    static Shell instance;
    return instance;
}

QQmlApplicationEngine* engine()
{
    if (shell().engine == nullptr) {
        auto* created = new QQmlApplicationEngine;
        created->addImageProvider(QStringLiteral("lucide"), new LucideProvider);
        created->addImageProvider(QStringLiteral("theme"), new ThemeProvider);
        shell().engine = created;
    }
    return shell().engine;
}

void openWindow(const QString& folder, const QString& activation)
{
    QQmlApplicationEngine* windows = engine();
    windows->setInitialProperties({ { QStringLiteral("startFolder"), folder.isEmpty() ? shell().home : folder } });
    windows->load(QUrl(QStringLiteral("qrc:/qt/qml/Rustphin/qml/Main.qml")));
    const QList<QObject*> roots = windows->rootObjects();
    if (auto* window = roots.isEmpty() ? nullptr : qobject_cast<QWindow*>(roots.last())) {
        // Qt's Wayland support hands this token to KWin with the request.
        if (!activation.isEmpty()) {
            qputenv("XDG_ACTIVATION_TOKEN", activation.toUtf8());
        }
        window->requestActivate();
    }
}

// The windows and the QML engine with them, all at once. Once the engine
// is gone, the C library hands the freed memory back to the system, which
// it otherwise keeps for later allocations (R-020).
void releaseWindows()
{
    if (shell().engine != nullptr) {
        QObject::connect(shell().engine, &QObject::destroyed, [] {
            // The render threads finish releasing their windows after the
            // engine is gone; trim once they have.
            QTimer::singleShot(1000, [] {
                QPixmapCache::clear();
                malloc_trim(0);
            });
        });
        shell().engine->deleteLater();
        shell().engine = nullptr;
    }
}

void lastWindowClosed()
{
    if (shell().keepRunning) {
        releaseWindows();
    } else {
        QCoreApplication::quit();
    }
}

} // namespace

void rustphin_shell_start(bool keep_running, const QString& home)
{
    shell().keepRunning = keep_running;
    shell().home = home;
    QDirIterator fonts(QStringLiteral(":/qt/qml/Rustphin/fonts"), { QStringLiteral("*.ttf") });
    while (fonts.hasNext()) {
        QFontDatabase::addApplicationFont(fonts.next());
    }
    // KDE's platform theme names the desktop's icon theme; elsewhere use
    // Breeze, as the GPUI interface does.
    if (QIcon::themeName().isEmpty() || QIcon::themeName() == QStringLiteral("hicolor")) {
        QIcon::setThemeName(QStringLiteral("breeze-dark"));
    }
    QGuiApplication::setQuitOnLastWindowClosed(false);
    QObject::connect(qGuiApp, &QGuiApplication::lastWindowClosed, qGuiApp, &lastWindowClosed);
}

void rustphin_shell_open(const QStringList& folders, const QString& activation)
{
    QMetaObject::invokeMethod(
        qGuiApp,
        [folders, activation] {
            if (folders.isEmpty()) {
                openWindow(QString(), activation);
            }
            for (const QString& folder : folders) {
                openWindow(folder, activation);
            }
        },
        Qt::QueuedConnection);
}

void rustphin_shell_quit()
{
    QMetaObject::invokeMethod(
        qGuiApp,
        [] {
            releaseWindows();
            QCoreApplication::quit();
        },
        Qt::QueuedConnection);
}

void rustphin_post(rust::Fn<void()> run)
{
    QMetaObject::invokeMethod(qGuiApp, [run] { run(); }, Qt::QueuedConnection);
}
