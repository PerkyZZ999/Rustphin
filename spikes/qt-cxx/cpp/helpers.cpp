#include "helpers.h"

#include <KWindowEffects>
#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtGui/QWindow>

bool spike_blur()
{
    const bool available = KWindowEffects::isEffectAvailable(KWindowEffects::BlurBehind);
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        KWindowEffects::enableBlurBehind(window, true);
    }
    return available;
}

void spike_copy(const QStringList& paths, bool cut)
{
    QList<QUrl> urls;
    for (const QString& path : paths) {
        urls.append(QUrl::fromLocalFile(path));
    }
    auto* data = new QMimeData;
    data->setUrls(urls);
    if (cut) {
        data->setData(QStringLiteral("application/x-kde-cutselection"), "1");
    }
    QGuiApplication::clipboard()->setMimeData(data);
}

QStringList spike_paste()
{
    QStringList paths;
    const QMimeData* data = QGuiApplication::clipboard()->mimeData();
    if (data == nullptr) {
        return paths;
    }
    for (const QUrl& url : data->urls()) {
        if (url.isLocalFile()) {
            paths.append(url.toLocalFile());
        }
    }
    return paths;
}

void spike_keep_running()
{
    QGuiApplication::setQuitOnLastWindowClosed(false);
}
