use keepass_ng::db::IconId;
use std::cell::RefCell;
use std::collections::{HashMap, hash_map::Entry};
use wxdragon::Bitmap;

thread_local! {
    static ICONS: RefCell<HashMap<(String, u32), Bitmap>> = RefCell::new(HashMap::new());
}

pub fn bitmap_for(icon_id: &IconId, size: u32) -> Option<Bitmap> {
    icon_for_emoji(&icon_id.to_string(), size)
}

pub(crate) fn icon_for_emoji(name: &str, size: u32) -> Option<Bitmap> {
    if size == 0 {
        return None;
    }
    let name = name.trim();

    ICONS.with(|icons| {
        let key = (name.to_owned(), size);
        let mut icons = icons.borrow_mut();
        match icons.entry(key) {
            Entry::Occupied(bitmap) => Some(bitmap.get().clone()),
            Entry::Vacant(entry) => {
                let bitmap = create_bitmap(name, size)?;
                Some(entry.insert(bitmap).clone())
            }
        }
    })
}

pub(crate) fn warm_up_step(size: u32, index: usize) -> bool {
    let Ok(icon_id) = IconId::try_from(index) else {
        return false;
    };
    let _ = bitmap_for(&icon_id, size);
    true
}

fn create_bitmap(icon: &str, size: u32) -> Option<Bitmap> {
    let center = size / 2;
    let baseline = size.saturating_mul(17) / 20;
    let font_size = size.saturating_mul(4) / 5;
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}">
<text x="{center}" y="{baseline}" text-anchor="middle" font-family="Segoe UI Emoji, Apple Color Emoji, Noto Color Emoji, Noto Emoji" font-size="{font_size}">{icon}</text>
</svg>"#
    );
    let mut options = resvg::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &options).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Bitmap::from_rgba(pixmap.data(), size, size)
}
