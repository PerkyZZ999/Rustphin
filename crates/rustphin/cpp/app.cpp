#include "app.h"
#include "window.h"

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

// The window last given the keyboard, for tabs other launches open.
QWindow* lastActive()
{
    if (shell().engine == nullptr) {
        return nullptr;
    }
    QWindow* focused = QGuiApplication::focusWindow();
    QWindow* found = nullptr;
    for (QObject* root : shell().engine->rootObjects()) {
        auto* window = qobject_cast<QWindow*>(root);
        if (window == nullptr || !window->isVisible()) {
            continue;
        }
        if (window == focused) {
            return window;
        }
        found = window;
    }
    return found;
}

void activate(QWindow* window, const QString& activation)
{
    // Qt's Wayland support hands this token to KWin with the request.
    if (!activation.isEmpty()) {
        qputenv("XDG_ACTIVATION_TOKEN", activation.toUtf8());
    }
    window->requestActivate();
}

// A window with a tab on each folder, or with none, the saved tabs when no
// window is open (the start window, which saves them), else home. Names
// in `select` are selected in the first folder.
QWindow* openWindow(const QStringList& folders, const QString& activation,
    const QStringList& select = {})
{
    // The first window restores and saves the session; folders named at
    // launch replace its tabs.
    const bool start = lastActive() == nullptr;
    QQmlApplicationEngine* windows = engine();
    windows->setInitialProperties({
        { QStringLiteral("startFolders"), folders },
        { QStringLiteral("startSelect"), select },
        { QStringLiteral("startWindow"), start },
        { QStringLiteral("home"), shell().home },
    });
    windows->load(QUrl(QStringLiteral("qrc:/qt/qml/Rustphin/qml/Main.qml")));
    const QList<QObject*> roots = windows->rootObjects();
    auto* window = roots.isEmpty() ? nullptr : qobject_cast<QWindow*>(roots.last());
    if (window != nullptr) {
        // A window closed while another stays open is freed with its
        // panes and models; the last one goes with the engine (R-020).
        QObject::connect(window, &QWindow::visibleChanged, window, [window](bool visible) {
            if (!visible && lastActive() != nullptr) {
                window->deleteLater();
            }
        });
        activate(window, activation);
    }
    return window;
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
    // KWindowSystem learns whether KWin offers blur one turn of the event
    // loop after it is first asked. Ask now, so the answer is there when the
    // first window, opened from the loop, chooses its theme.
    (void)rustphin_blur_available();
}

void rustphin_shell_keep_running(bool keep_running)
{
    shell().keepRunning = keep_running;
}

void rustphin_shell_open(const QStringList& folders, const QString& activation)
{
    QMetaObject::invokeMethod(
        qGuiApp, [folders, activation] { openWindow(folders, activation); }, Qt::QueuedConnection);
}

void rustphin_shell_show(const QStringList& folders, const QString& activation, bool in_tabs)
{
    QWindow* window = in_tabs && !folders.isEmpty() ? lastActive() : nullptr;
    if (window == nullptr) {
        openWindow(folders, activation);
        return;
    }
    QMetaObject::invokeMethod(window, "openTabs", Q_ARG(QStringList, folders));
    activate(window, activation);
}

void rustphin_shell_reveal(const QString& folder, const QStringList& select, bool in_tabs,
    const QString& activation)
{
    QWindow* window = in_tabs ? lastActive() : nullptr;
    if (window == nullptr) {
        openWindow({ folder }, activation, select);
        return;
    }
    QMetaObject::invokeMethod(window, "reveal", Q_ARG(QString, folder), Q_ARG(QStringList, select));
    activate(window, activation);
}

void rustphin_shell_properties(const QStringList& paths, const QString& activation)
{
    QWindow* window = lastActive();
    if (window == nullptr) {
        window = openWindow({}, activation);
    } else {
        activate(window, activation);
    }
    if (window != nullptr) {
        QMetaObject::invokeMethod(window, "openDialog",
            Q_ARG(QString, QStringLiteral("properties")),
            Q_ARG(QVariant, QVariant(paths)));
    }
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

int rustphin_keyboard_modifiers()
{
    return static_cast<int>(QGuiApplication::queryKeyboardModifiers().toInt());
}

void rustphin_post(rust::Fn<void()> run)
{
    QMetaObject::invokeMethod(qGuiApp, [run] { run(); }, Qt::QueuedConnection);
}
