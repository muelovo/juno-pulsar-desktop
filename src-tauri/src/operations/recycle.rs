use super::policy::{inspect, Identity};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use windows::{
    core::*,
    Win32::{Foundation::*, System::Com::*, UI::Shell::*},
};
#[implement(IFileOperationProgressSink)]
struct Sink {
    path: PathBuf,
    snapshot: Identity,
    result: Arc<Mutex<Option<bool>>>,
}
impl IFileOperationProgressSink_Impl for Sink_Impl {
    fn StartOperations(&self) -> Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _: HRESULT) -> Result<()> {
        Ok(())
    }
    fn PreDeleteItem(&self, flags: u32, _: Ref<'_, IShellItem>) -> Result<()> {
        if flags & TSF_DELETE_RECYCLE_IF_POSSIBLE.0 as u32 == 0
            || inspect(&self.path).ok().as_ref() != Some(&self.snapshot)
        {
            return Err(Error::from_hresult(E_ACCESSDENIED));
        }
        Ok(())
    }
    fn PostDeleteItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        hr: HRESULT,
        recycled: Ref<'_, IShellItem>,
    ) -> Result<()> {
        *self
            .result
            .lock()
            .map_err(|_| Error::from_hresult(E_FAIL))? = Some(hr.is_ok() && !recycled.is_null());
        Ok(())
    }
    fn PreRenameItem(&self, _: u32, _: Ref<'_, IShellItem>, _: &PCWSTR) -> Result<()> {
        Err(Error::from_hresult(E_ACCESSDENIED))
    }
    fn PostRenameItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<'_, IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreMoveItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
    ) -> Result<()> {
        Err(Error::from_hresult(E_ACCESSDENIED))
    }
    fn PostMoveItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<'_, IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreCopyItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
    ) -> Result<()> {
        Err(Error::from_hresult(E_ACCESSDENIED))
    }
    fn PostCopyItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
        _: HRESULT,
        _: Ref<'_, IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn PreNewItem(&self, _: u32, _: Ref<'_, IShellItem>, _: &PCWSTR) -> Result<()> {
        Err(Error::from_hresult(E_ACCESSDENIED))
    }
    fn PostNewItem(
        &self,
        _: u32,
        _: Ref<'_, IShellItem>,
        _: &PCWSTR,
        _: &PCWSTR,
        _: u32,
        _: HRESULT,
        _: Ref<'_, IShellItem>,
    ) -> Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, _: u32, _: u32) -> Result<()> {
        Ok(())
    }
    fn ResetTimer(&self) -> Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> Result<()> {
        Ok(())
    }
}
pub fn recycle(path: PathBuf, snapshot: Identity) -> Result<bool> {
    // SAFETY: fresh dedicated thread owns the apartment and all COM values until return.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            // SAFETY: balanced on initializing thread.
            unsafe { CoUninitialize() }
        }
    }
    let _apt = Apartment;
    let outcome = Arc::new(Mutex::new(None));
    let sink: IFileOperationProgressSink = Sink {
        path: path.clone(),
        snapshot,
        result: outcome.clone(),
    }
    .into();
    // SAFETY: interfaces live in this apartment. UTF16 path lives through item creation.
    unsafe {
        let op: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)?;
        op.SetOperationFlags(
            FOFX_RECYCLEONDELETE
                | FOFX_ADDUNDORECORD
                | FOFX_EARLYFAILURE
                | FOFX_NOCOPYHOOKS
                | FOF_NOERRORUI
                | FOF_SILENT
                | FOF_NOCONFIRMATION,
        )?;
        let wide = HSTRING::from(path.as_os_str());
        let item: IShellItem = SHCreateItemFromParsingName(&wide, None)?;
        op.DeleteItem(&item, &sink)?;
        op.PerformOperations()?;
        if op.GetAnyOperationsAborted()?.as_bool() {
            return Ok(false);
        }
    }
    let result = *outcome.lock().map_err(|_| Error::from_hresult(E_FAIL))?;
    Ok(result == Some(true))
}
