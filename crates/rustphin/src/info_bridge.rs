//! The cxx-qt bridge for the Properties window's object: declarations only.

pub use super::PropertiesModelRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// Text.
        type QString = cxx_qt_lib::QString;
    }

    // Properties are written by the Rust side only; QML binds to them.
    extern "RustQt" {
        /// One entry's properties in four tabs (W-114f, W-170); `view` is
        /// everything the tabs show, as JSON.
        #[qobject]
        #[qml_element]
        #[qproperty(i32, window)]
        #[qproperty(QString, title)]
        #[qproperty(QString, kind)]
        #[qproperty(QString, icon)]
        #[qproperty(QString, theme_icon, cxx_name = "themeIcon")]
        #[qproperty(QString, thumbnail)]
        #[qproperty(QString, view)]
        #[qproperty(QString, error)]
        #[qproperty(bool, loading)]
        #[qproperty(bool, dirty)]
        type PropertiesModel = super::PropertiesModelRust;
    }

    unsafe extern "RustQt" {
        /// Reads the entry at the `file:` URI `uri` in the background.
        #[qinvokable]
        fn open(self: Pin<&mut PropertiesModel>, uri: &QString);
        /// Turns one permission on or off: `group` (the class) 0–2, `which` 0 read,
        /// 1 write, 2 execute.
        #[qinvokable]
        #[cxx_name = "setBit"]
        fn set_bit(self: Pin<&mut PropertiesModel>, group: i32, which: i32, on: bool);
        /// Gives the class `who` (0 owner, 1 group, 2 others) the access
        /// choice `index` (`ACCESS_CHOICES` order).
        #[qinvokable]
        #[cxx_name = "chooseAccess"]
        fn choose_access(self: Pin<&mut PropertiesModel>, who: i32, index: i32);
        /// Allows running the file as a program, or not.
        #[qinvokable]
        #[cxx_name = "setRuns"]
        fn set_runs(self: Pin<&mut PropertiesModel>, on: bool);
        /// Chooses the group at `index` of the view's group list.
        #[qinvokable]
        #[cxx_name = "chooseGroup"]
        fn choose_group(self: Pin<&mut PropertiesModel>, index: i32);
        /// Saves changed permissions and group through the operation engine.
        #[qinvokable]
        fn apply(self: Pin<&mut PropertiesModel>);
        /// Calculates the checksum at `index` (MD5, SHA1, SHA256, SHA512).
        #[qinvokable]
        fn calculate(self: Pin<&mut PropertiesModel>, index: i32);
        /// Stops calculating the checksum at `index`.
        #[qinvokable]
        #[cxx_name = "stopSum"]
        fn stop_sum(self: Pin<&mut PropertiesModel>, index: i32);
        /// The text of "Check a download".
        #[qinvokable]
        fn paste(self: Pin<&mut PropertiesModel>, text: &QString);
        /// Shows the type's default application again (after Change…).
        #[qinvokable]
        #[cxx_name = "refreshApp"]
        fn refresh_app(self: Pin<&mut PropertiesModel>);
        /// Stops measuring a folder and calculating checksums.
        #[qinvokable]
        fn close(self: Pin<&mut PropertiesModel>);
        /// Shows the measuring's progress.
        fn tick(self: Pin<&mut PropertiesModel>);
    }

    impl cxx_qt::Threading for PropertiesModel {}
}
