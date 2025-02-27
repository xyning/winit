#![cfg(wayland_platform)]

//! Winit's Wayland backend.

use std::fmt::Display;
use std::sync::Arc;

use sctk::reexports::client::globals::{BindError, GlobalError};
use sctk::reexports::client::protocol::wl_surface::WlSurface;
use sctk::reexports::client::{self, ConnectError, DispatchError, Proxy};

pub(super) use crate::cursor::OnlyCursorImage as CustomCursor;
use crate::dpi::{LogicalSize, PhysicalSize};
pub use crate::platform_impl::platform::{OsError, WindowId};
pub use event_loop::{EventLoop, EventLoopProxy, EventLoopWindowTarget};
pub use output::{MonitorHandle, VideoModeHandle};
pub use window::Window;

mod event_loop;
mod output;
mod seat;
mod state;
mod types;
mod window;

#[derive(Debug)]
pub enum WaylandError {
    /// Error connecting to the socket.
    Connection(ConnectError),

    /// Error binding the global.
    Global(GlobalError),

    // Bind error.
    Bind(BindError),

    /// Error during the dispatching the event queue.
    Dispatch(DispatchError),

    /// Calloop error.
    Calloop(calloop::Error),

    /// Wayland
    Wire(client::backend::WaylandError),
}

impl Display for WaylandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaylandError::Connection(error) => error.fmt(f),
            WaylandError::Global(error) => error.fmt(f),
            WaylandError::Bind(error) => error.fmt(f),
            WaylandError::Dispatch(error) => error.fmt(f),
            WaylandError::Calloop(error) => error.fmt(f),
            WaylandError::Wire(error) => error.fmt(f),
        }
    }
}

impl From<WaylandError> for OsError {
    fn from(value: WaylandError) -> Self {
        Self::WaylandError(Arc::new(value))
    }
}

/// Dummy device id, since Wayland doesn't have device events.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId;

impl DeviceId {
    pub const unsafe fn dummy() -> Self {
        DeviceId
    }
}

/// Get the WindowId out of the surface.
#[inline]
fn make_wid(surface: &WlSurface) -> WindowId {
    WindowId(surface.id().as_ptr() as u64)
}

/// The default routine does floor, but we need round on Wayland.
fn logical_to_physical_rounded(size: LogicalSize<u32>, scale_factor: f64) -> PhysicalSize<u32> {
    let width = size.width as f64 * scale_factor;
    let height = size.height as f64 * scale_factor;
    (width.round(), height.round()).into()
}

#[derive(Debug)]
pub enum GenericPointer {
    Default(sctk::seat::pointer::ThemedPointer<seat::WinitPointerData>),
    Tablet(crate::platform_impl::wayland::seat::TabletPointer),
}
impl GenericPointer {
    fn set_cursor(
        &self,
        conn: &wayland_client::Connection,
        icon: cursor_icon::CursorIcon,
    ) -> Result<(), sctk::seat::pointer::PointerThemeError> {
        match self {
            GenericPointer::Default(pointer) => pointer.set_cursor(conn, icon),
            GenericPointer::Tablet(pointer) => pointer.set_cursor(conn, icon),
        }
    }
    fn surface(&self) -> &WlSurface {
        match self {
            GenericPointer::Default(pointer) => pointer.surface(),
            GenericPointer::Tablet(pointer) => pointer.surface(),
        }
    }
    fn set_cursor_raw(
        &self,
        serial: u32,
        surface: Option<&WlSurface>,
        hotspot_x: i32,
        hotspot_y: i32,
    ) {
        match self {
            crate::platform_impl::wayland::GenericPointer::Default(pointer) => {
                pointer
                    .pointer()
                    .set_cursor(serial, surface, hotspot_x, hotspot_y);
            }
            crate::platform_impl::wayland::GenericPointer::Tablet(pointer) => {
                pointer
                    .tool()
                    .set_cursor(serial, surface, hotspot_x, hotspot_y);
            }
        }
    }
    fn clear_cursor(&self) {
        match self {
            crate::platform_impl::wayland::GenericPointer::Default(pointer) => {
                pointer
                    .pointer()
                    .set_cursor(self.winit_data().latest_enter_serial(), None, 0, 0);
            }
            crate::platform_impl::wayland::GenericPointer::Tablet(pointer) => {
                pointer
                    .tool()
                    .set_cursor(self.winit_data().latest_enter_serial(), None, 0, 0);
            }
        }
    }
    fn winit_data(&self) -> &seat::WinitPointerData {
        match self {
            GenericPointer::Default(pointer) => {
                seat::WinitPointerDataExt::winit_data(pointer.pointer())
            }
            GenericPointer::Tablet(pointer) => pointer.winit_data(),
        }
    }
}
impl PartialEq for GenericPointer {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Default(a), Self::Default(b)) => a.pointer() == b.pointer(),
            (Self::Tablet(a), Self::Tablet(b)) => a.tool() == b.tool(),
            _ => false,
        }
    }
}
