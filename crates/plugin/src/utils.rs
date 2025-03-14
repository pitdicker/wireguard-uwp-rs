//! Utilities and helper types that don't quite fit anywhere else.

use std::sync::atomic::{AtomicU32, Ordering};

use windows::{
    core::{implement, AsImpl, Error, Interface, Ref, Result, RuntimeType, Type, IUnknownImpl},
    Networking::Vpn::VpnPacketBuffer,
    Win32::Foundation::{E_BOUNDS, E_NOTIMPL},
    Win32::System::WinRT::IBufferByteAccess,
};
use windows_collections::{
    IIterable, IIterable_Impl, IIterator, IIterator_Impl, IVector, IVectorView, IVectorView_Impl,
    IVector_Impl,
};

/// A simple wrapper around `Vec` which implements the `IVector`, `IVectorView` and
/// `IIterable` interfaces.
#[implement(
    IIterable<T>,
    IVector<T>,
    IVectorView<T>
)]
pub struct Vector<T>(Vec<T::Default>)
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone;

impl<T> IVector_Impl<T> for Vector_Impl<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    fn GetAt(&self, index: u32) -> Result<T> {
        self.0
            .get(index as usize)
            .map(|el| T::from_default(el))
            .transpose()?
            .ok_or(Error::from(E_BOUNDS))
    }

    fn Size(&self) -> Result<u32> {
        u32::try_from(self.0.len()).map_err(|_| Error::from(E_BOUNDS))
    }

    fn GetView(&self) -> Result<IVectorView<T>> {
        Ok(self.to_interface::<IVectorView<T>>())
    }

    fn IndexOf(&self, value: Ref<'_, T>, index: &mut u32) -> Result<bool> {
        if let Some(idx) = self.0.iter().position(|el| *el == *value) {
            *index = u32::try_from(idx).map_err(|_| Error::from(E_BOUNDS))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn SetAt(&self, _index: u32, _value: Ref<'_, T>) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn InsertAt(&self, _index: u32, _value: Ref<'_, T>) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn RemoveAt(&self, _index: u32) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Append(&self, _value: Ref<'_, T>) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn RemoveAtEnd(&self) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Clear(&self) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn GetMany(&self, start: u32, items: &mut [T::Default]) -> Result<u32> {
        let sz = u32::try_from(self.0.len()).map_err(|_| Error::from(E_BOUNDS))?;

        if start >= sz {
            return Err(Error::from(E_BOUNDS));
        }

        let mut count = 0;
        for (item, el) in items.into_iter().zip(self.0[start as usize..].iter()) {
            *item = el.clone();
            count += 1;
        }
        Ok(count)
    }

    fn ReplaceAll(&self, _values: &[T::Default]) -> Result<()> {
        Err(E_NOTIMPL.into())
    }
}

impl<T> IVectorView_Impl<T> for Vector_Impl<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    fn GetAt(&self, index: u32) -> Result<T> {
        self.0
            .get(index as usize)
            .map(|el| T::from_default(el))
            .transpose()?
            .ok_or(Error::from(E_BOUNDS))
    }

    fn Size(&self) -> Result<u32> {
        u32::try_from(self.0.len()).map_err(|_| Error::from(E_BOUNDS))
    }

    fn IndexOf(&self, value: Ref<T>, index: &mut u32) -> Result<bool> {
        if let Some(idx) = self.0.iter().position(|el| *el == *value) {
            *index = u32::try_from(idx).map_err(|_| Error::from(E_BOUNDS))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn GetMany(&self, start: u32, items: &mut [T::Default]) -> Result<u32> {
        let sz = u32::try_from(self.0.len()).map_err(|_| Error::from(E_BOUNDS))?;

        if start >= sz {
            return Err(Error::from(E_BOUNDS));
        }

        let mut count = 0;
        for (item, el) in items.into_iter().zip(self.0[start as usize..].iter()) {
            *item = el.clone();
            count += 1;
        }
        Ok(count)
    }
}

impl<T> IIterable_Impl<T> for Vector_Impl<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    fn First(&self) -> Result<IIterator<T>> {
        Ok(VectorIterator::<T> {
            it: self.to_interface::<IIterable<T>>(),
            curr: AtomicU32::new(0),
        }
        .into())
    }
}

impl<T> Vector<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    pub fn new(v: Vec<T::Default>) -> IVector<T> {
        Vector(v).into()
    }
}

/// `IIterator` wrapper for `Vector`
#[implement(IIterator<T>)]
struct VectorIterator<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    /// The underlying object we're iteratoring over
    it: IIterable<T>,
    /// The current position of the iterator
    curr: AtomicU32,
}

impl<T> IIterator_Impl<T> for VectorIterator_Impl<T>
where
    T: RuntimeType + 'static,
    <T as Type<T>>::Default: PartialEq + Clone,
{
    fn Current(&self) -> Result<T> {
        let vec = self.it.cast::<IVector<T>>().expect("unexpected type");
        vec.GetAt(self.curr.load(Ordering::Relaxed))
    }

    fn HasCurrent(&self) -> Result<bool> {
        let vec: &Vector<T> = unsafe { self.it.as_impl() };
        Ok(vec.0.len() > self.curr.load(Ordering::Relaxed) as usize)
    }

    fn MoveNext(&self) -> Result<bool> {
        let vec: &Vector<T> = unsafe { self.it.as_impl() };
        let old = self.curr.fetch_add(1, Ordering::Relaxed) as usize;
        Ok(vec.0.len() > old + 1)
    }

    fn GetMany(&self, items: &mut [T::Default]) -> Result<u32> {
        let vec = self.it.cast::<IVector<T>>().expect("unexpected type");
        vec.GetMany(0, items)
    }
}

pub trait IBufferExt {
    /// Get a slice to an `IBuffer`'s underlying buffer.
    fn get_buf(&self) -> Result<&[u8]>;

    /// Get a mutable slice to an `IBuffer`'s underlying buffer.
    fn get_buf_mut(&mut self) -> Result<&mut [u8]>;
}

impl IBufferExt for VpnPacketBuffer {
    fn get_buf(&self) -> Result<&[u8]> {
        let buffer = self.Buffer()?;
        let len = buffer.Length()?;
        let rawBuffer = buffer.cast::<IBufferByteAccess>()?;
        Ok(unsafe {
            // SAFETY: Any type that implements `IBuffer` must also implement `IBufferByteAccess`
            // to return the buffer as an array of bytes.
            std::slice::from_raw_parts(rawBuffer.Buffer()?, len as usize)
        })
    }

    fn get_buf_mut(&mut self) -> Result<&mut [u8]> {
        let buffer = self.Buffer()?;
        let len = buffer.Length()?;
        let rawBuffer = buffer.cast::<IBufferByteAccess>()?;
        Ok(unsafe {
            // SAFETY: Any type that implements `IBuffer` must also implement `IBufferByteAccess`
            // to return the buffer as an array of bytes.
            std::slice::from_raw_parts_mut(rawBuffer.Buffer()?, len as usize)
        })
    }
}

macro_rules! debug_log {
    ($fmt:tt) => {
        unsafe {
            use ::windows::core::PCSTR;
            use ::windows::Win32::System::Diagnostics::Debug::OutputDebugStringA;
            let mut msg = format!(concat!($fmt, "\n\0"));
            OutputDebugStringA(PCSTR(msg.as_mut_ptr()));
        }
    };
    ($fmt:tt, $($arg:tt)*) => {
        unsafe {
            use ::windows::core::PCSTR;
            use ::windows::Win32::System::Diagnostics::Debug::OutputDebugStringA;
            let mut msg = format!(concat!($fmt, "\n\0"), $($arg)*);
            OutputDebugStringA(PCSTR(msg.as_mut_ptr()));
        }
    };
}

pub(crate) use debug_log;
