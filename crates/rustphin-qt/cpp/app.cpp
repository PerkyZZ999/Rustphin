#include "app.h"

#include <QtCore/QDirIterator>
#include <QtCore/QFile>
#include <QtCore/QUrl>
#include <QtGui/QColor>
#include <QtGui/QFontDatabase>
#include <QtGui/QIcon>
#include <QtGui/QImage>
#include <QtGui/QPainter>
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

} // namespace

void rustphin_install(QQmlApplicationEngine& engine)
{
    QDirIterator fonts(QStringLiteral(":/qt/qml/Rustphin/fonts"), { QStringLiteral("*.ttf") });
    while (fonts.hasNext()) {
        QFontDatabase::addApplicationFont(fonts.next());
    }
    // KDE's platform theme names the desktop's icon theme; elsewhere use
    // Breeze, as the GPUI interface does.
    if (QIcon::themeName().isEmpty() || QIcon::themeName() == QStringLiteral("hicolor")) {
        QIcon::setThemeName(QStringLiteral("breeze-dark"));
    }
    engine.addImageProvider(QStringLiteral("lucide"), new LucideProvider);
    engine.addImageProvider(QStringLiteral("theme"), new ThemeProvider);
}
