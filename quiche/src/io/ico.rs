#[derive(Debug)]
pub enum IcoError {
    InvalidDimensions(String),
    InvalidLength(String),
    TooManyEntries(String),
    InvalidFileType(String),
    InvalidResourceType(String),
    UnexpectedValue(String),
    UnknownColorDepth(String),
    InvalidBitmap(String),
    UnsupportedFileType(String),
    BufferOutOfBounds(String),
}

#[allow(non_snake_case)]
impl std::fmt::Display for IcoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            IcoError::InvalidDimensions(ref s) => write!(f, "{}", s),
            IcoError::InvalidLength(ref s) => write!(f, "{}", s),
            IcoError::TooManyEntries(ref s) => write!(f, "{}", s),
            IcoError::InvalidFileType(ref s) => write!(f, "{}", s),
            IcoError::InvalidResourceType(ref s) => write!(f, "{}", s),
            IcoError::UnexpectedValue(ref s) => write!(f, "{}", s),
            IcoError::UnknownColorDepth(ref s) => write!(f, "{}", s),
            IcoError::InvalidBitmap(ref s) => write!(f, "{}", s),
            IcoError::UnsupportedFileType(ref s) => write!(f, "{}", s),
            IcoError::BufferOutOfBounds(ref s) => write!(f, "{}", s),
        }
    }
}

/// .ico files must have a minimum width of 16
const MIN_WIDTH: u32 = 16;
/// .ico files have a maximum width of 256
const MAX_WIDTH: u32 = 256;
/// .ico files must have a minimum height of 16
const MIN_HEIGHT: u32 = 16;
/// .ico files have a maximum height of 256
const MAX_HEIGHT: u32 = 256;
/// the ICO file format has reserved offsets which are supposed to be zero.
/// although Microsoft's technical documentation states that this value must be zero, the icon encoder
/// built into .NET (System.Drawing.Icon.Save) sets this value to 255.
/// it appears that the operating system ignores this value altogether.
const RESERVED: u8 = 0;

/// Extensions for creating a simple binary reader/writer
/// without relying on a huge crate
pub trait VecExt {
    fn read_u32(&self, offset: &mut usize) -> Result<u32, IcoError>;
    fn read_i32(&self, offset: &mut usize) -> Result<i32, IcoError>;
    fn read_u16(&self, offset: &mut usize) -> Result<u16, IcoError>;
    fn read_u8(&self, offset: &mut usize) -> Result<u8, IcoError>;
    fn write_u32(&mut self, value: u32);
    fn write_i32(&mut self, value: i32);
    fn write_u16(&mut self, value: u16);
    fn write_u8(&mut self, value: u8);
    fn write_bytes(&mut self, value: &Vec<u8>);
    fn slice(&self, offset: usize, length: usize) -> Vec<u8>;
}

impl VecExt for Vec<u8> {
    /// Reads a 4-byte unsigned integer from the current vector
    /// and advances the position of the cursor by four bytes.
    fn read_u32(&self, offset: &mut usize) -> Result<u32, IcoError> {
        let size = std::mem::size_of::<u32>();
        if *offset + size > self.len() {
            return Err(IcoError::BufferOutOfBounds(format!("Provided offset {} will read passed the bounds of the buffer for data with the length of {}", *offset, size)));
        }
        let result = ((self[*offset] as u32) << 0)
            | ((self[*offset + 1] as u32) << 8)
            | ((self[*offset + 2] as u32) << 16)
            | ((self[*offset + 3] as u32) << 24);
        *offset += size;
        Ok(result)
    }
    /// Reads a 4-byte signed integer from the current vector
    /// and advances the current position of the cursor by four bytes.
    fn read_i32(&self, offset: &mut usize) -> Result<i32, IcoError> {
        let size = std::mem::size_of::<i32>();
        if *offset + size > self.len() {
            return Err(IcoError::BufferOutOfBounds(format!("Provided offset {} will read passed the bounds of the buffer for data with the length of {}", *offset, size)));
        }
        let result = ((self[*offset] as i32) << 0)
            | ((self[*offset + 1] as i32) << 8)
            | ((self[*offset + 2] as i32) << 16)
            | ((self[*offset + 3] as i32) << 24);
        *offset += 4;
        Ok(result)
    }

    /// Reads a 2-byte unsigned integer from the current vector using little-endian encoding
    /// and advances the position of the cursor by two bytes.
    fn read_u16(&self, offset: &mut usize) -> Result<u16, IcoError> {
        let size = std::mem::size_of::<u16>();
        if *offset + size > self.len() {
            return Err(IcoError::BufferOutOfBounds(format!("Provided offset {} will read passed the bounds of the buffer for data with the length of {}", *offset, size)));
        }
        let result = (self[*offset] as u16) | (self[*offset + 1] as u16);
        *offset += 2;
        Ok(result)
    }
    /// Reads the next byte from the current vector
    /// and advances the current position of the cursor by one byte.
    fn read_u8(&self, offset: &mut usize) -> Result<u8, IcoError> {
        let size = std::mem::size_of::<u8>();
        if *offset + size > self.len() {
            return Err(IcoError::BufferOutOfBounds(format!("Provided offset {} will read passed the bounds of the buffer for data with the length of {}", *offset, size)));
        }
        let result = self[*offset];
        *offset += 1;
        Ok(result)
    }

    /// Reads the specified number of bytes from the current vector into a `Vec<u8>`
    /// and advances the current position by that number of bytes.
    fn slice(&self, offset: usize, length: usize) -> Vec<u8> {
        self[offset..offset + length].to_vec()
    }
    /// Writes a four-byte unsigned integer to the current vector
    /// and advances the cursor position by four bytes.
    fn write_u32(&mut self, value: u32) {
        let data = &[
            (value >> 0) as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
        ];
        self.extend(data);
    }
    /// Writes a four-byte signed integer to the current vector
    /// and advances the cursor position by four bytes.
    fn write_i32(&mut self, value: i32) {
        let data = &[
            (value >> 0) as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
        ];
        self.extend(data);
    }
    /// Writes a two-byte unsigned integer to the current vector
    /// and advances the cursor position by two bytes.
    fn write_u16(&mut self, value: u16) {
        let data = &[(value >> 0) as u8, (value >> 8) as u8];
        self.extend(data);
    }
    /// Writes an unsigned byte to the current vector
    /// and advances the cursor position by one byte.
    fn write_u8(&mut self, value: u8) {
        self.push(value);
    }
    /// Writes a `Vec<u8>` to the underlying vector.
    fn write_bytes(&mut self, value: &Vec<u8>) {
        self.extend(value);
    }
}
pub struct IconImage {
    /// the bitmap header information associated with the image
    pub header: BitmapInfoHeader,
    /// the raw pixel data of the bitmap
    pub data: Vec<u8>,
}

impl IconImage {
    /// returns the width of the underlying bitmap
    pub fn get_width(&self) -> u32 {
        self.header.width
    }
    /// returns the height of the underlying bitmap
    pub fn get_height(&self) -> u32 {
        self.header.height
    }
}

impl IconImage {
    /// Creates a new image from the provided bitmap information.
    /// must have `4 * width * height` bytes and be in row-major order from top to bottom.
    pub fn from_pixel_data(header: BitmapInfoHeader, data: Vec<u8>) -> Result<IconImage, IcoError> {
        if header.width < MIN_WIDTH || header.width > MAX_WIDTH {
            return Err(IcoError::InvalidDimensions(format!(
                "Invalid width (was {}, but range is {}-{})",
                header.width, MIN_WIDTH, MAX_WIDTH
            )));
        }
        if header.height < MIN_HEIGHT || header.height > MAX_HEIGHT {
            return Err(IcoError::InvalidDimensions(format!(
                "Invalid height (was {}, but range is {}-{})",
                header.height, MIN_HEIGHT, MAX_HEIGHT
            )));
        }
        let expected_data_len = (4 * header.width * header.height) as usize;
        if data.len() != expected_data_len {
            return Err(IcoError::InvalidLength(format!(
                "Invalid data length \
                 (was {}, but must be {} for {}x{} image)",
                data.len(),
                expected_data_len,
                header.width,
                header.height
            )));
        }
        Ok(IconImage { header, data })
    }

    pub fn read_bmp(data: &Vec<u8>) -> Result<IconImage, IcoError> {
        let mut cursor = 0 as usize;
        let bitmap = BitmapInfoHeader::new(&data, &mut cursor)?;

        let depth = bitmap.get_depth()?;

        let mut colors = Vec::<IconColor>::with_capacity(depth.num_colors());
        for _ in 0..depth.num_colors() {
            let blue = data.read_u8(&mut cursor)?;
            let green = data.read_u8(&mut cursor)?;
            let red = data.read_u8(&mut cursor)?;
            let _reserved = data.read_u8(&mut cursor)?;
            colors.push(IconColor {
                red: red,
                green: green,
                blue: blue,
            });
        }

        let num_pixels = (bitmap.width * bitmap.height * 4) as usize;
        let mut rgba = vec![std::u8::MAX; num_pixels];
        let row_data_size = (bitmap.width * bitmap.bits_per_pixel as u32 + 7) / 8;
        let row_padding_size = ((row_data_size + 3) / 4) * 4 - row_data_size;

        for row in 0..bitmap.height {
            let mut start = (4 * (bitmap.height - row - 1) * bitmap.width) as usize;
            match depth {
                ColorDepth::One => {
                    let mut col = 0;
                    for _ in 0..row_data_size {
                        let byte = data.read_u8(&mut cursor)?;
                        for bit in 0..8 {
                            let index = (byte >> (7 - bit)) & 0x1;
                            let color = &colors[index as usize];
                            rgba[start] = color.red;
                            rgba[start + 1] = color.green;
                            rgba[start + 2] = color.blue;
                            col += 1;
                            if col == bitmap.width {
                                break;
                            }
                            start += 4;
                        }
                    }
                }
                ColorDepth::Four => {
                    let mut col = 0;
                    for _ in 0..row_data_size {
                        let byte = data.read_u8(&mut cursor)?;
                        for nibble in 0..2 {
                            let index = (byte >> (4 * (1 - nibble))) & 0xf;
                            let color = &colors[index as usize];
                            rgba[start] = color.red;
                            rgba[start + 1] = color.green;
                            rgba[start + 2] = color.blue;
                            col += 1;
                            if col == bitmap.width {
                                break;
                            }
                            start += 4;
                        }
                    }
                }
                ColorDepth::Eight => {
                    for _ in 0..bitmap.width {
                        let index = data.read_u8(&mut cursor)?;
                        let color = &colors[index as usize];
                        rgba[start] = color.red;
                        rgba[start + 1] = color.green;
                        rgba[start + 2] = color.blue;
                        start += 4;
                    }
                }
                ColorDepth::Sixteen => {
                    for _ in 0..bitmap.width {
                        let color = data.read_u16(&mut cursor)?;
                        let red = (color >> 10) & 0x1f;
                        let green = (color >> 5) & 0x1f;
                        let blue = color & 0x1f;
                        rgba[start] = ((red * 255 + 15) / 31) as u8;
                        rgba[start + 1] = ((green * 255 + 15) / 31) as u8;
                        rgba[start + 2] = ((blue * 255 + 15) / 31) as u8;
                        start += 4;
                    }
                }
                ColorDepth::TwentyFour => {
                    for _ in 0..bitmap.width {
                        let blue = data.read_u8(&mut cursor)?;
                        let green = data.read_u8(&mut cursor)?;
                        let red = data.read_u8(&mut cursor)?;
                        rgba[start] = red;
                        rgba[start + 1] = green;
                        rgba[start + 2] = blue;
                        start += 4;
                    }
                }
                ColorDepth::ThirtyTwo => {
                    for _ in 0..bitmap.width {
                        let blue = data.read_u8(&mut cursor)?;
                        let green = data.read_u8(&mut cursor)?;
                        let red = data.read_u8(&mut cursor)?;
                        let alpha = data.read_u8(&mut cursor)?;
                        rgba[start] = red;
                        rgba[start + 1] = green;
                        rgba[start + 2] = blue;
                        rgba[start + 3] = alpha;
                        start += 4;
                    }
                }
            }
            if row_padding_size > 0 {
                cursor += row_padding_size as usize;
            }
        }

        if depth != ColorDepth::ThirtyTwo {
            let row_mask_size = (bitmap.width + 7) / 8;
            let row_padding_size = ((row_mask_size + 3) / 4) * 4 - row_mask_size;
            for row in 0..bitmap.height {
                let mut start = (4 * (bitmap.height - row - 1) * bitmap.width) as usize;
                let mut col = 0;
                for _ in 0..row_mask_size {
                    let byte = data.read_u8(&mut cursor)?;
                    for bit in 0..8 {
                        if ((byte >> (7 - bit)) & 0x1) == 1 {
                            rgba[start + 3] = 0;
                        }
                        col += 1;
                        if col == bitmap.width {
                            break;
                        }
                        start += 4;
                    }
                }
                if row_padding_size > 0 {
                    cursor += row_padding_size as usize;
                }
            }
        }
        IconImage::from_pixel_data(bitmap, rgba)
    }
}

/// The RGB information of a given pixel
pub struct IconColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// An entry in an Icon directory
pub struct IconDirEntry {
    /// width of the icon
    pub width: u8,
    /// height of the icon
    pub height: u8,
    /// number of colors in the icon.
    /// will always be zero if 8 bits per pixel.
    colors: u8,
    /// the color planes
    color_planes: u16,
    /// the number of bits per pixel
    bits_per_pixel: u16,
    data: Vec<u8>,
}

impl IconDirEntry {
    /// returns true if the entry is encoded as a PNG.
    pub fn is_png(&self) -> bool {
        self.data.starts_with(&[0x89, b'P', b'N', b'G'])
    }
    /// decodes the entry into RGBA raw pixels.
    pub fn decode(&self) -> Result<IconImage, IcoError> {
        if self.is_png() {
            return Err(IcoError::UnsupportedFileType(String::from(
                "PNG based ICO files are not currently supported.",
            )));
        }
        let image = IconImage::read_bmp(&self.data)?;
        if image.get_width() != self.width as u32 || image.get_height() != self.height as u32 {
            return Err(IcoError::InvalidDimensions(format!(
                "Encoded image has wrong dimensions \
                 (was {}x{}, but should be {}x{})",
                image.get_width(),
                image.get_height(),
                self.width,
                self.height
            )));
        }
        Ok(image)
    }
}

pub struct IconDir {
    /// the resource type of the icon directory
    pub resource_type: ResourceType,
    ///
    pub entries: Vec<IconDirEntry>,
}

/// A collection of images stored inside an ICO file
impl IconDir {
    /// Takes an existing `IconDir` and encodes it to a valid .ico file.
    /// This is useful if you add or remove entries from the file.
    pub fn encode(&self) -> Result<Vec<u8>, IcoError> {
        let mut buffer: Vec<u8> = Vec::new();
        if self.entries.len() > (std::u16::MAX as usize) {
            return Err(IcoError::TooManyEntries(format!(
                "Too many entries in IconDir \
                 (was {}, but max is {})",
                self.entries.len(),
                std::u16::MAX
            )));
        }
        buffer.write_u16(RESERVED.into());
        buffer.write_u16(self.resource_type.number());
        buffer.write_u16(self.entries.len() as u16);
        let mut data_offset = 6 + 16 * (self.entries.len() as u32);
        for entry in self.entries.iter() {
            buffer.write_u8(entry.width);
            buffer.write_u8(entry.height);
            buffer.write_u8(entry.colors);
            buffer.write_u8(RESERVED);
            buffer.write_u16(entry.color_planes);
            buffer.write_u16(entry.bits_per_pixel);
            buffer.write_u32(entry.data.len() as u32);
            buffer.write_u32(data_offset);
            data_offset += entry.data.len() as u32;
        }
        for entry in self.entries.iter() {
            buffer.write_bytes(&entry.data);
        }
        Ok(buffer)
    }
    /// Creates an `IconDir` from a `Vec<u8>` that contains the full contents of a .ico file
    pub fn from(ico_file_data: &Vec<u8>) -> Result<IconDir, IcoError> {
        let mut cursor = 0 as usize;
        if ico_file_data.read_u16(&mut cursor)? != RESERVED as u16 {
            return Err(IcoError::InvalidFileType(String::from(
                "Could not find ICO header.",
            )));
        }
        let resource_type = ResourceType::from_number(ico_file_data.read_u16(&mut cursor)?)?;
        if resource_type == ResourceType::Cursor {
            return Err(IcoError::UnsupportedFileType(String::from(
                "cursor files are currently not supported.",
            )));
        }

        let total_entries = ico_file_data.read_u16(&mut cursor)? as usize;
        let mut entries = Vec::<IconDirEntry>::with_capacity(total_entries);
        for _ in 0..total_entries {
            let width = ico_file_data.read_u8(&mut cursor)?;
            let height = ico_file_data.read_u8(&mut cursor)?;
            let total_colors = ico_file_data.read_u8(&mut cursor)?;
            let reserved = ico_file_data.read_u8(&mut cursor)?;
            if reserved != RESERVED {
                return Err(IcoError::UnexpectedValue(format!(
                    "Expected RESERVED value. Found {}",
                    reserved
                )));
            }
            let color_planes = ico_file_data.read_u16(&mut cursor)?;
            let bits_per_pixel = ico_file_data.read_u16(&mut cursor)?;
            let entry_size = ico_file_data.read_u32(&mut cursor)? as usize;
            let entry_offset = ico_file_data.read_u32(&mut cursor)? as usize;
            entries.push(IconDirEntry {
                width: width,
                height: height,
                colors: total_colors,
                color_planes: color_planes,
                bits_per_pixel: bits_per_pixel,
                data: ico_file_data.slice(entry_offset, entry_size),
            });
        }
        Ok(IconDir {
            resource_type: resource_type,
            entries: entries,
        })
    }
}

/// the type of the resource inside the .ico file.
#[derive(PartialEq)]
pub enum ResourceType {
    /// plain images (ICO files)
    Icon,
    /// images with cursor hotspots (CUR files)
    Cursor,
}

impl ResourceType {
    pub fn from_number(number: u16) -> Result<ResourceType, IcoError> {
        match number {
            1 => Ok(ResourceType::Icon),
            2 => Ok(ResourceType::Cursor),
            _ => Err(IcoError::InvalidResourceType(format!(
                "Could not locate a valid ICO resource type: {}",
                number
            ))),
        }
    }

    pub fn number(&self) -> u16 {
        match *self {
            ResourceType::Icon => 1,
            ResourceType::Cursor => 2,
        }
    }
}

#[derive(PartialEq)]
enum ColorDepth {
    One,
    Four,
    Eight,
    Sixteen,
    TwentyFour,
    ThirtyTwo,
}

impl ColorDepth {
    fn num_colors(&self) -> usize {
        match *self {
            ColorDepth::One => 2,
            ColorDepth::Four => 16,
            ColorDepth::Eight => 256,
            _ => 0,
        }
    }
}

pub struct BitmapInfoHeader {
    /// Specifies the number of bytes required by the structure.
    /// This value does not include the size of the color table
    /// or the size of the color masks, if they are appended to the end of structure.
    pub size: u32,
    /// the width of the bitmap, in pixels.
    pub width: u32,
    /// the height of the bitmap, in pixels.
    pub height: u32,
    /// the number of planes for the target device. This value must be set to 1.
    pub planes: u16,
    /// the number of bits per pixel (bpp).
    /// for uncompressed formats, this value is the average number of bits per pixel.
    /// for compressed formats, this value is the implied bit depth of the uncompressed image, after the image has been decoded.
    pub bits_per_pixel: u16,
    /// for compressed video and YUV formats, this member is a FOURCC code, specified as a DWORD in little-endian order.
    /// for example, YUYV video has the FOURCC `VYUY` or `0x56595559`.
    pub compression: u32,
    /// the size, in bytes, of the image. This can be set to 0 for uncompressed RGB bitmaps.
    pub image_size: i32,
    /// specifies the horizontal resolution, in pixels per meter, of the target device for the bitmap.
    pub x_pixels: u32,
    /// specifies the vertical resolution, in pixels per meter, of the target device for the bitmap.
    pub y_pixels: u32,
    /// specifies the number of color indices in the color table that are actually used by the bitmap.
    pub colors_used: u32,
    /// specifies the number of color indices that are considered important for displaying the bitmap.
    /// if this value is zero, all colors are important.
    pub colors_important: u32,
}

impl BitmapInfoHeader {
    fn get_depth(&self) -> Result<ColorDepth, IcoError> {
        match self.bits_per_pixel {
            1 => Ok(ColorDepth::One),
            4 => Ok(ColorDepth::Four),
            8 => Ok(ColorDepth::Eight),
            16 => Ok(ColorDepth::Sixteen),
            24 => Ok(ColorDepth::TwentyFour),
            32 => Ok(ColorDepth::ThirtyTwo),
            _ => Err(IcoError::UnknownColorDepth(format!(
                "unknown ColorDepth ({})",
                self.bits_per_pixel
            ))),
        }
    }

    /// Reads the Bitmap header from the `IconDirEntry`
    pub fn new(data: &Vec<u8>, cursor: &mut usize) -> Result<BitmapInfoHeader, IcoError> {
        let size = data.read_u32(cursor)?;
        if size != std::mem::size_of::<BitmapInfoHeader>() as u32 {
            return Err(IcoError::InvalidBitmap(format!(
                "bitmap header size is incorrect. correct size of {} != read size of {}",
                std::mem::size_of::<BitmapInfoHeader>(),
                size
            )));
        }
        let width = data.read_u32(cursor)?;
        if width < (MIN_WIDTH) || width > (MAX_WIDTH) {
            return Err(IcoError::InvalidDimensions(format!(
                "Invalid BMP width (was {}, but range is {}-{})",
                width, MIN_WIDTH, MAX_WIDTH
            )));
        }
        let height = data.read_u32(cursor)?;
        if height % 2 != 0 {
            // The height is stored doubled, counting the rows of both the
            // color data and the alpha mask, so it should be divisible by 2.
            return Err(IcoError::InvalidDimensions(format!(
                "Invalid height field in BMP header \
                 (was {}, but must be divisible by 2)",
                height
            )));
        }
        let height = height / 2;
        if height < (MIN_HEIGHT) || height > (MAX_HEIGHT) {
            return Err(IcoError::InvalidDimensions(format!(
                "Invalid BMP height (was {}, but range is {}-{})",
                height, MIN_HEIGHT, MAX_HEIGHT
            )));
        }
        let planes = data.read_u16(cursor)?;
        let bits_per_pixel = data.read_u16(cursor)?;
        let compression = data.read_u32(cursor)?;
        let image_size = data.read_i32(cursor)?;
        let x_pixels = data.read_u32(cursor)?;
        let y_pixels = data.read_u32(cursor)?;
        let colors_used = data.read_u32(cursor)?;
        let colors_important = data.read_u32(cursor)?;

        Ok(BitmapInfoHeader {
            size,
            width,
            height,
            planes,
            bits_per_pixel,
            compression,
            image_size,
            x_pixels,
            y_pixels,
            colors_used,
            colors_important,
        })
    }
}