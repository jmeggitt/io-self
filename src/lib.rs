use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io;
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::sync::Arc;

pub mod grammar;
pub mod helper;
pub mod positional;

#[doc(hidden)]
pub mod derive_util;

use helper::AbortingFromIterator;
pub use positional::PositionAware;

pub trait ReadSelf: Sized {
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self>;

    fn from_bytes<B: AsRef<[u8]>>(bytes: &B) -> io::Result<Self> {
        let mut buffer = Cursor::new(bytes.as_ref());
        Self::read_from(&mut buffer)
    }

    fn read_exact_from<F: FromIterator<Self>, B: Read + PositionAware>(
        buffer: &mut B,
        count: usize,
    ) -> io::Result<F> {
        (0..count)
            .into_iter()
            .map(|_| Self::read_from(buffer))
            .aborting_from_iter()
    }
}

impl ReadSelf for u8 {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        buffer.read_u8()
    }
}

impl ReadSelf for i8 {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        buffer.read_i8()
    }
}

impl<T: ?Sized> ReadSelf for PhantomData<T> {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(_: &mut B) -> io::Result<Self> {
        Ok(PhantomData)
    }
}

impl ReadSelf for () {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(_: &mut B) -> io::Result<Self> {
        Ok(())
    }
}

impl<T: ReadSelf> ReadSelf for Box<T> {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        Ok(Box::new(T::read_from(buffer)?))
    }
}

impl<T: ReadSelf> ReadSelf for Arc<T> {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        Ok(Arc::new(T::read_from(buffer)?))
    }
}

impl<T: ReadSelf> ReadSelf for Rc<T> {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        Ok(Rc::new(T::read_from(buffer)?))
    }
}

impl<T: ReadSelf, const N: usize> ReadSelf for [T; N] {
    #[inline(always)]
    fn read_from<B: Read + PositionAware>(buffer: &mut B) -> io::Result<Self> {
        unsafe {
            let mut array = MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init();

            for item in array.iter_mut().take(N) {
                item.write(T::read_from(buffer)?);
            }

            Ok((&array as *const _ as *const [T; N]).read())
        }
    }
}

#[doc(hidden)]
macro_rules! impl_read_tuple {
    ($($generic:ident)*) => {
        impl<$($generic: ?Sized + ReadSelf),*> ReadSelf for ($($generic),*) {
            #[inline(always)]
            fn read_from<Buf: Read + PositionAware>(buffer: &mut Buf) -> io::Result<Self> {
                Ok(($(<$generic as ReadSelf>::read_from(buffer)?),*))
            }
        }

        impl<$($generic: ?Sized + WriteSelf),*> WriteSelf for ($($generic),*) {
            #[inline(always)]
            fn write_to<Buf: Write + PositionAware>(&self, buffer: &mut Buf) -> io::Result<()> {
                #[allow(non_snake_case)]
                let ($($generic),*) = self;
                $(<$generic as WriteSelf>::write_to($generic, buffer)?;)*
                Ok(())
            }
        }
    };
}

impl_read_tuple! {A B}
impl_read_tuple! {A B C}
impl_read_tuple! {A B C D}
impl_read_tuple! {A B C D E}
impl_read_tuple! {A B C D E F}
impl_read_tuple! {A B C D E F G}
impl_read_tuple! {A B C D E F G H}
impl_read_tuple! {A B C D E F G H I}
impl_read_tuple! {A B C D E F G H I J}
impl_read_tuple! {A B C D E F G H I J K}
impl_read_tuple! {A B C D E F G H I J K L}
impl_read_tuple! {A B C D E F G H I J K L M}
impl_read_tuple! {A B C D E F G H I J K L M N}

/// At times it may be desirable to be able to reuse existing memory without needing to copy
/// `Self`. This variant works similarly to `ReadSelf`, but reads into an existing instance of
/// `Self`.
pub trait ReadIntoSelf {
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()>;
}

impl<T: ReadIntoSelf> ReadIntoSelf for [T] {
    #[inline(always)]
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()> {
        self.iter_mut().read_into(buffer)
    }
}

impl<'a, T: 'a + ReadIntoSelf, I: Iterator<Item = &'a mut T>> ReadIntoSelf for I {
    #[inline(always)]
    fn read_into<B: Read + PositionAware>(&mut self, buffer: &mut B) -> io::Result<()> {
        for item in self {
            item.read_into(buffer)?;
        }
        Ok(())
    }
}

pub trait WriteSelf: Sized {
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()>;
}

impl WriteSelf for u8 {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        buffer.write_u8(*self)
    }
}

impl WriteSelf for i8 {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        buffer.write_i8(*self)
    }
}

impl<T: ?Sized> WriteSelf for PhantomData<T> {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, _: &mut B) -> io::Result<()> {
        Ok(())
    }
}

impl WriteSelf for () {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, _: &mut B) -> io::Result<()> {
        Ok(())
    }
}

impl<T: ?Sized + WriteSelf> WriteSelf for Box<T> {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        T::write_to(&*self, buffer)
    }
}

impl<T: ?Sized + WriteSelf> WriteSelf for Arc<T> {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        T::write_to(&*self, buffer)
    }
}

impl<T: ?Sized + WriteSelf> WriteSelf for Rc<T> {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        T::write_to(&*self, buffer)
    }
}

impl<T: WriteSelf, const N: usize> WriteSelf for [T; N] {
    #[inline(always)]
    fn write_to<B: Write + PositionAware>(&self, buffer: &mut B) -> io::Result<()> {
        for item in self {
            item.write_to(buffer)?;
        }
        Ok(())
    }
}
