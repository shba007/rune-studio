use tauri::{Manager, Runtime, WebviewWindow, WebviewWindowBuilder};

pub trait BuilderShim: Sized {
    fn always_on_top(self, always_on_top: bool) -> Self;
    fn decorations(self, decorations: bool) -> Self;
    fn center(self) -> Self;
    fn shadow(self, shadow: bool) -> Self;
    fn skip_taskbar(self, skip: bool) -> Self;
    fn focused(self, focused: bool) -> Self;
    fn focusable(self, focusable: bool) -> Self;
}

impl<R: Runtime, M: Manager<R>> BuilderShim for WebviewWindowBuilder<'_, R, M> {
    fn always_on_top(self, _always_on_top: bool) -> Self {
        self
    }
    fn decorations(self, _decorations: bool) -> Self {
        self
    }
    fn center(self) -> Self {
        self
    }
    fn shadow(self, _shadow: bool) -> Self {
        self
    }
    fn skip_taskbar(self, _skip: bool) -> Self {
        self
    }
    fn focused(self, _focused: bool) -> Self {
        self
    }
    fn focusable(self, _focusable: bool) -> Self {
        self
    }
}

pub trait WindowShim {
    fn unminimize(&self) -> tauri::Result<()>;
    fn unmaximize(&self) -> tauri::Result<()>;
    fn center(&self) -> tauri::Result<()>;
    fn set_always_on_top(&self, always_on_top: bool) -> tauri::Result<()>;
    fn set_ignore_cursor_events(&self, ignore: bool) -> tauri::Result<()>;
}

impl<R: Runtime> WindowShim for WebviewWindow<R> {
    fn unminimize(&self) -> tauri::Result<()> {
        Ok(())
    }
    fn unmaximize(&self) -> tauri::Result<()> {
        Ok(())
    }
    fn center(&self) -> tauri::Result<()> {
        Ok(())
    }
    fn set_always_on_top(&self, _always_on_top: bool) -> tauri::Result<()> {
        Ok(())
    }
    fn set_ignore_cursor_events(&self, _ignore: bool) -> tauri::Result<()> {
        Ok(())
    }
}
