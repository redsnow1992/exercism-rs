use std::{
    borrow::Borrow,
    io::{Read, Write},
};

/// A munger which XORs a key with some data
#[derive(Clone)]
pub struct Xorcism<'a> {
    idx: usize, // next idx to access key
    key: &'a [u8],
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    ///
    /// Should accept anything which has a cheap conversion to a byte slice.
    pub fn new<Key>(key: &'a Key) -> Xorcism<'a>
    where
        Key: AsRef<[u8]> + ?Sized,
    {
        Self {
            idx: 0,
            key: key.as_ref(),
        }
    }

    /// XOR each byte of the input buffer with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    pub fn munge_in_place(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.xor_inplace(byte);
        }
    }

    fn advance(&mut self) {
        self.idx += 1;

        if self.idx >= self.key.len() {
            self.idx = 0;
        }
    }

    fn xor_inplace(&mut self, byte: &mut u8) {
        *byte ^= self.key[self.idx];
        self.advance();
    }

    fn xor(&mut self, byte: &u8) -> u8 {
        let ret = *byte ^ self.key[self.idx];
        self.advance();
        ret
    }

    /// XOR each byte of the data with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    ///
    /// Should accept anything which has a cheap conversion to a byte iterator.
    /// Shouldn't matter whether the byte iterator's values are owned or borrowed.
    pub fn munge<Data, T>(&mut self, data: Data) -> impl Iterator<Item = u8>
    where
        Data: IntoIterator<Item = T>,
        T: Borrow<u8>,
    {
        let data: Vec<u8> = data
            .into_iter()
            .map(|byte| self.xor(byte.borrow()))
            .collect();

        XorData { data, cur_idx: 0 }
    }

    pub fn reader(self, reader: impl Read + 'a) -> impl Read + 'a {
        XorDataReader {
            xor: self,
            data: reader,
        }
    }

    pub fn writer(self, writer: impl Write + 'a) -> impl Write + 'a {
        XorDataWriter {
            xor: self,
            data: writer,
        }
    }
}

struct XorData {
    data: Vec<u8>,
    cur_idx: usize,
}

impl Iterator for XorData {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_idx < self.data.len() {
            let ret = self.data[self.cur_idx];
            self.cur_idx += 1;

            Some(ret)
        } else {
            None
        }
    }
}

struct XorDataReader<'a, DataReader> {
    xor: Xorcism<'a>,
    data: DataReader,
}
impl<'a, DataReader> Read for XorDataReader<'a, DataReader>
where
    DataReader: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let i = self.data.read(buf)?;
        self.xor.munge_in_place(buf);

        Ok(i)
    }
}

struct XorDataWriter<'a, DataWriter> {
    xor: Xorcism<'a>,
    data: DataWriter,
}

impl<'a, DataWriter> Write for XorDataWriter<'a, DataWriter>
where
    DataWriter: Write,
{
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let buf = self.xor.munge(input).collect::<Vec<_>>();
        self.data.write(&buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
