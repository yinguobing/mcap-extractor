use ros2_std_msgs::msg::Header;
use serde::Deserialize;

/// This message contains a compressed image.
///
/// std_msgs/Header header
/// Header timestamp should be acquisition time of image
/// Header frame_id should be optical frame of camera
/// origin of frame should be optical center of camera
/// +x should point to the right in the image
/// +y should point down in the image
/// +z should point into to plane of the image
///
/// string format
/// Specifies the format of the data
/// Acceptable values:
/// jpeg, png
///
/// uint8[] data
/// Compressed image buffer
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub struct CompressedImage {
    pub header: Header,
    pub format: String,
    pub data: Vec<u8>,
}

impl CompressedImage {
    pub fn new(header: Header, format: String, data: Vec<u8>) -> Self {
        CompressedImage {
            header,
            format,
            data,
        }
    }

    pub fn name() -> &'static str {
        "CompressedImage"
    }
}
