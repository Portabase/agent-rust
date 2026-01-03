use std::path::Path;

pub fn full_extension(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            match name.find('.') {
                Some(idx) => &name[idx..],
                None => "",
            }
        })
        .unwrap_or_default()
        .to_string()
}

