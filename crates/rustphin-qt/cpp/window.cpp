#include "window.h"

#include <KWindowEffects>
#include <QtGui/QGuiApplication>
#include <QtGui/QWindow>

bool rustphin_blur_available()
{
    return KWindowEffects::isEffectAvailable(KWindowEffects::BlurBehind);
}

void rustphin_blur_windows()
{
    for (QWindow* window : QGuiApplication::topLevelWindows()) {
        KWindowEffects::enableBlurBehind(window, true);
    }
}
