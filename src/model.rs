use crate::connection::Connection;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Rectangle {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) connections: Vec<Connection>,
}

impl Rectangle {
    pub(crate) fn valid(&self) -> bool {
        [self.x, self.y, self.z, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width >= 1.0
            && self.height >= 1.0
    }

    pub(crate) fn fit_text(&mut self) {
        let width = self
            .text
            .split('\n')
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        self.width = self.width.max((width + 2) as f64);
        self.height = self.height.max((self.text.split('\n').count() + 2) as f64);
    }

    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}
