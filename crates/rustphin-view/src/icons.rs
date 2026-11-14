//! The interface's icons, named as in Lucide and free of any toolkit
//! (W-114b2). Each interface maps an [`Icon`] to its own drawing: GPUI-Kit's
//! bundled set, or the Lucide SVG file [`Icon::lucide_name`] names.

/// Calls `$callback!` with every icon the interface uses, so each toolkit
/// builds its mapping from the same list.
#[macro_export]
macro_rules! for_each_icon {
    ($callback:ident) => {
        $callback! {
            AppWindow,
            Archive,
            ArchiveRestore,
            ArchiveX,
            ArrowDown,
            ArrowDownAZ,
            ArrowDownWideNarrow,
            ArrowLeft,
            ArrowLeftRight,
            ArrowRight,
            ArrowUp,
            ArrowUpFromLine,
            Bookmark,
            BookmarkMinus,
            BookmarkPlus,
            CalendarClock,
            Check,
            ChevronDown,
            ChevronRight,
            ChevronsDownUp,
            ChevronsUpDown,
            CircleCheck,
            CircleStop,
            CircleX,
            ClipboardPaste,
            Clock,
            Columns3,
            Command,
            Compass,
            Copy,
            CopyPlus,
            Download,
            Eye,
            EyeOff,
            File,
            FileArchive,
            FileCode,
            FilePlus,
            FileText,
            FileType,
            Film,
            Folder,
            FolderArchive,
            FolderInput,
            FolderOpen,
            FolderOutput,
            FolderPlus,
            FolderX,
            Funnel,
            FunnelX,
            Gamepad2,
            HardDrive,
            Hash,
            House,
            Image,
            Info,
            Keyboard,
            LayoutGrid,
            LayoutList,
            Link,
            List,
            ListPlus,
            Loader,
            Lock,
            LockKeyhole,
            Menu,
            Merge,
            Minus,
            Monitor,
            Move,
            Music,
            PackageOpen,
            Palette,
            PanelLeft,
            PanelsTopLeft,
            PanelTop,
            Pencil,
            Play,
            Plus,
            Power,
            RefreshCw,
            Replace,
            RotateCcw,
            RotateCw,
            Scissors,
            Search,
            SearchX,
            Server,
            Settings,
            Shield,
            ShieldAlert,
            ShieldOff,
            SkipForward,
            SlidersHorizontal,
            Square,
            SquareArrowOutUpRight,
            SquareCheck,
            SquarePlus,
            SquareSplitHorizontal,
            SquareTerminal,
            Terminal,
            TextCursorInput,
            Trash,
            TriangleAlert,
            Type,
            Undo2,
            Usb,
            X,
            ZoomIn,
            ZoomOut,
        }
    };
}

macro_rules! define_icons {
    ($($name:ident,)*) => {
        /// An icon of the interface, by its Lucide name.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Icon {
            $(
                #[doc = concat!("Lucide's `", stringify!($name), "`.")]
                $name,
            )*
        }

        impl Icon {
            /// Every icon, in the list's order.
            pub const ALL: &'static [Icon] = &[$(Icon::$name,)*];

            /// The icon's name as written in code (`ArrowDownAZ`).
            pub fn code_name(self) -> &'static str {
                match self {
                    $(Icon::$name => stringify!($name),)*
                }
            }
        }
    };
}

for_each_icon!(define_icons);

impl Icon {
    /// Lucide's file name for the icon, without `.svg`: `ArrowDownAZ` is
    /// `arrow-down-a-z`, `Columns3` is `columns-3`.
    pub fn lucide_name(self) -> String {
        let mut name = String::new();
        let mut previous_digit = false;
        for (index, character) in self.code_name().chars().enumerate() {
            let digit = character.is_ascii_digit();
            if index > 0 && (character.is_ascii_uppercase() || (digit && !previous_digit)) {
                name.push('-');
            }
            name.push(character.to_ascii_lowercase());
            previous_digit = digit;
        }
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lucide_names_follow_lucides_file_names() {
        assert_eq!(Icon::ArrowDownAZ.lucide_name(), "arrow-down-a-z");
        assert_eq!(Icon::Columns3.lucide_name(), "columns-3");
        assert_eq!(
            Icon::SquareArrowOutUpRight.lucide_name(),
            "square-arrow-out-up-right"
        );
        assert_eq!(Icon::Undo2.lucide_name(), "undo-2");
        assert_eq!(Icon::X.lucide_name(), "x");
    }
}
