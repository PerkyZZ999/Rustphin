#![no_main]

use std::path::Path;

use libfuzzer_sys::fuzz_target;
use rustphin_platform::places::Places;

fuzz_target!(|data: &str| {
    let Ok(places) = Places::parse(data) else {
        return;
    };
    let original: Vec<_> = places.places().cloned().collect();
    if let Ok((added, id)) = places.with_added("Fuzz <&\"é", Path::new("/fuzz"), "folder", 0) {
        let added = Places::parse(&added).unwrap();
        assert_eq!(added.places().count(), original.len() + 1);
        let removed = added.without(&id).unwrap();
        assert_eq!(
            Places::parse(&removed)
                .unwrap()
                .places()
                .cloned()
                .collect::<Vec<_>>(),
            original
        );
    }
    if let Some(place) = places.places().next() {
        if let Ok(renamed) = places.with_title(&place.id, "Fuzz <&\"é") {
            let renamed = Places::parse(&renamed).unwrap();
            assert_eq!(renamed.places().next().unwrap().title, "Fuzz <&\"é");
        }
        if let Ok(hidden) = places.with_hidden(&place.id, true) {
            assert!(
                Places::parse(&hidden)
                    .unwrap()
                    .places()
                    .next()
                    .unwrap()
                    .hidden
            );
        }
    }
});
