#pragma once

#include <QtQml/QQmlApplicationEngine>

// Registers the bundled fonts, the icon theme fallback and the two image
// providers QML draws icons with:
//   image://lucide/<name>/<aarrggbb>  a bundled Lucide icon in that colour
//   image://theme/<name>|<name>…      the first icon the theme has
void rustphin_install(QQmlApplicationEngine& engine);
