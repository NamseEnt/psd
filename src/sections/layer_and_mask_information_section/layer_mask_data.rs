use super::PsdLayerError;
use crate::PsdCursor;

const POSITION_RELATIVE_TO_LAYER: u8 = 0b0000_0001;
const LAYER_MASK_DISABLED: u8 = 0b0000_0010;
const INVERT_LAYER_MASK_WHEN_BLENDING: u8 = 0b0000_0100;
const USER_MASK_CAME_FROM_RENDERING_OTHER_DATA: u8 = 0b0000_1000;
const MASKS_HAVE_PARAMETERS_APPLIED: u8 = 0b0001_0000;

const USER_MASK_DENSITY: u8 = 0b0000_0001;
const USER_MASK_FEATHER: u8 = 0b0000_0010;
const VECTOR_MASK_DENSITY: u8 = 0b0000_0100;
const VECTOR_MASK_FEATHER: u8 = 0b0000_1000;

#[derive(Debug, Clone)]
pub struct LayerMaskData {
    pub vector_mask: Option<LayerMaskDataInner>,
    pub raster_mask: Option<LayerMaskDataInner>,
}

#[derive(Debug, Clone)]
pub struct LayerMaskDataInner {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
    pub default_color: u8,
    pub flags: u8,
    pub density: u8,
    pub feather: f64,
}
impl LayerMaskDataInner {
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/#50577409_26431
/// See Layer mask / adjustment layer data for structure. Can be 40 bytes, 24 bytes, or 4 bytes if no layer mask.
///
/// Flags:
/// - bit 0 = position relative to layer
/// - bit 1 = layer mask disabled
/// - bit 2 = invert layer mask when blending (Obsolete)
/// - bit 3 = indicates that the user mask actually came from rendering other data
/// - bit 4 = indicates that the user and/or vector masks have parameters applied to them
pub fn read_layer_mask_data(cursor: &mut PsdCursor) -> Result<LayerMaskData, PsdLayerError> {
    let layer_mask_data_len = cursor.read_u32();
    if layer_mask_data_len < 16 {
        cursor.read(layer_mask_data_len);
        return Ok(LayerMaskData {
            vector_mask: None,
            raster_mask: None,
        });
    }
    let mut read_count = 0;

    let first_mask = {
        let top = cursor.read_i32();
        let left = cursor.read_i32();
        let bottom = cursor.read_i32();
        let right = cursor.read_i32();
        read_count += 4 * 4;

        let default_color = cursor.read_u8();
        read_count += 1;

        let flags = cursor.read_u8();
        read_count += 1;

        LayerMaskDataInner {
            top,
            left,
            bottom,
            right,
            default_color,
            flags,
            density: 255,
            feather: 0.0,
        }
    };

    let second_mask = if layer_mask_data_len - read_count >= 18 {
        let flags = cursor.read_u8();
        read_count += 1;

        let default_color = cursor.read_u8();
        read_count += 1;

        let top = cursor.read_i32();
        let left = cursor.read_i32();
        let bottom = cursor.read_i32();
        let right = cursor.read_i32();
        read_count += 4 * 4;

        Some(LayerMaskDataInner {
            top,
            left,
            bottom,
            right,
            default_color,
            flags,
            density: 255,
            feather: 0.0,
        })
    } else {
        None
    };

    let mut raster_mask_density = 0;
    let mut raster_mask_feather = 0.0;
    let mut vector_mask_density = 0;
    let mut vector_mask_feather = 0.0;
    if first_mask.flags & MASKS_HAVE_PARAMETERS_APPLIED != 0 {
        let parameter_flags = cursor.read_u8();
        read_count += 1;

        if parameter_flags & USER_MASK_DENSITY != 0 {
            raster_mask_density = cursor.read_u8();
            read_count += 1;
        }

        if parameter_flags & USER_MASK_FEATHER != 0 {
            raster_mask_feather = cursor.read_f64();
            read_count += 8;
        }

        if parameter_flags & VECTOR_MASK_DENSITY != 0 {
            vector_mask_density = cursor.read_u8();
            read_count += 1;
        }

        if parameter_flags & VECTOR_MASK_FEATHER != 0 {
            vector_mask_feather = cursor.read_f64();
            read_count += 8;
        }
    }

    // Skip remaining bytes
    cursor.read(layer_mask_data_len - read_count);

    let mut layer_mask_data = if let Some(second_mask) = second_mask {
        LayerMaskData {
            vector_mask: Some(first_mask),
            raster_mask: Some(second_mask),
        }
    } else {
        if first_mask.user_mask_came_from_rendering_other_data() {
            LayerMaskData {
                vector_mask: Some(first_mask),
                raster_mask: None,
            }
        } else {
            LayerMaskData {
                vector_mask: None,
                raster_mask: Some(first_mask),
            }
        }
    };

    if let Some(raster_mask) = layer_mask_data.raster_mask.as_mut() {
        raster_mask.density = raster_mask_density;
        raster_mask.feather = raster_mask_feather;
    }
    if let Some(vector_mask) = layer_mask_data.vector_mask.as_mut() {
        vector_mask.density = vector_mask_density;
        vector_mask.feather = vector_mask_feather;
    }

    Ok(layer_mask_data)
}

impl LayerMaskDataInner {
    pub fn position_relative_to_layer(&self) -> bool {
        self.flags & POSITION_RELATIVE_TO_LAYER != 0
    }

    pub fn layer_mask_disabled(&self) -> bool {
        self.flags & LAYER_MASK_DISABLED != 0
    }

    pub fn invert_layer_mask_when_blending(&self) -> bool {
        self.flags & INVERT_LAYER_MASK_WHEN_BLENDING != 0
    }

    pub fn user_mask_came_from_rendering_other_data(&self) -> bool {
        self.flags & USER_MASK_CAME_FROM_RENDERING_OTHER_DATA != 0
    }

    pub fn user_and_or_vector_masks_have_parameters_applied(&self) -> bool {
        self.flags & MASKS_HAVE_PARAMETERS_APPLIED != 0
    }
}
