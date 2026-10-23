//! the plain-data format that what crosses between the world and whoever is watching or playing it is written in: the events that
//! come out of a frame and the input a player sends in. little-endian numbers with no padding, and every read is checked, so a
//! short or garbled message is `None` and never a panic

use crate::prelude::*;

//the most bytes of text that are written, the rest is cut off
pub(crate) const MAX_TEXT_BYTES: usize = 200;

#[derive(Default)]
pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn new() -> Writer {
        Writer::default()
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn flag(&mut self, value: bool) {
        self.bytes.push(value as u8);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn f32(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn vec2(&mut self, value: Vec2) {
        self.f32(value.x);
        self.f32(value.y);
    }

    //bytes as they are, for a fixed number of them the reader knows to ask for (see `Reader::array`)
    pub(crate) fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    //a whole number in as few bytes as it needs: seven bits to a byte, the top one set on every byte but the last
    pub(crate) fn varint(&mut self, mut value: u32) {
        while value >= 0x80 {
            self.bytes.push((value & 0x7f) as u8 | 0x80);
            value >>= 7;
        }
        self.bytes.push(value as u8);
    }

    //a stretch of bytes with its length in front, for a part of a message that is read on its own
    pub(crate) fn blob(&mut self, bytes: &[u8]) {
        self.u32(bytes.len() as u32);
        self.raw(bytes);
    }

    //a colour is four bytes, one for each of red, green, blue and alpha, so it is only kept to within 1/255 of what it was
    pub(crate) fn color(&mut self, value: Color) {
        for channel in [value.r, value.g, value.b, value.a] {
            self.u8((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }

    //text, cut short at MAX_TEXT_BYTES (where a letter ends)
    pub(crate) fn text(&mut self, value: &str) {
        let mut end = value.len().min(MAX_TEXT_BYTES);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.u8(end as u8);
        self.bytes.extend_from_slice(&value.as_bytes()[..end]);
    }
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { bytes }
    }

    //whether everything has been read
    pub(crate) fn is_done(&self) -> bool {
        self.bytes.is_empty()
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        if self.bytes.len() < count {
            return None;
        }
        let (taken, rest) = self.bytes.split_at(count);
        self.bytes = rest;
        Some(taken)
    }

    pub(crate) fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|bytes| bytes[0])
    }

    //a flag is 0 or 1, and nothing else is one
    pub(crate) fn flag(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    pub(crate) fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub(crate) fn u32(&mut self) -> Option<u32> {
        self.take(4).map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(crate) fn i32(&mut self) -> Option<i32> {
        self.u32().map(|value| value as i32)
    }

    //a number that is not a number (or is infinite) is refused, so nothing that comes in can poison the world with one
    pub(crate) fn f32(&mut self) -> Option<f32> {
        let value = f32::from_bits(self.u32()?);
        value.is_finite().then_some(value)
    }

    pub(crate) fn vec2(&mut self) -> Option<Vec2> {
        Some(Vec2::new(self.f32()?, self.f32()?))
    }

    pub(crate) fn color(&mut self) -> Option<Color> {
        let channel = |byte: u8| byte as f32 / 255.0;
        Some(Color::new(channel(self.u8()?), channel(self.u8()?), channel(self.u8()?), channel(self.u8()?)))
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N).map(|bytes| bytes.try_into().expect("N bytes were taken"))
    }

    //everything that is left, which is then all read
    pub(crate) fn rest(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.bytes)
    }

    //a number written by `Writer::varint`: one that runs on for more bytes than a number can have is not one
    pub(crate) fn varint(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let byte = self.u8()?;
            let part = (byte & 0x7f) as u32;
            if shift == 28 && part > 0x0f {
                return None;
            }
            value |= part << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    //a stretch of bytes written by `Writer::blob`, if it is no longer than `most` and all of it is there
    pub(crate) fn blob(&mut self, most: usize) -> Option<&'a [u8]> {
        let length = self.u32()? as usize;
        if length > most {
            return None;
        }
        self.take(length)
    }

    pub(crate) fn text(&mut self) -> Option<String> {
        let length = self.u8()? as usize;
        String::from_utf8(self.take(length)?.to_vec()).ok()
    }
}
