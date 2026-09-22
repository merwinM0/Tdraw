//! Backward-compatible JSON validation and atomic replacement of the canvas file.
use crate::model::Rectangle;
use std::{fs, io, path::Path};

pub(crate) fn load(path: &Path) -> io::Result<Vec<Rectangle>> {
    let rectangles: Vec<Rectangle> = match fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data).map_err(io::Error::other)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e),
    };
    if !rectangles.iter().all(Rectangle::valid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rects.json 包含无效坐标或尺寸",
        ));
    }
    if rectangles.iter().enumerate().any(|(i, r)| {
        r.connections.iter().any(|c| {
            c.target >= rectangles.len()
                || c.target == i
                || !c.from.valid()
                || !c.to.valid()
                || c.waypoints
                    .iter()
                    .any(|(x, y)| !x.is_finite() || !y.is_finite() || *x < 0.0 || *y < 0.0)
        })
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "rects.json 包含无效连接",
        ));
    }
    Ok(rectangles)
}

pub(crate) fn save(path: &Path, rectangles: &[Rectangle]) -> io::Result<()> {
    let data = serde_json::to_vec_pretty(rectangles).map_err(io::Error::other)?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, data)?;
    fs::rename(temp, path)
}
