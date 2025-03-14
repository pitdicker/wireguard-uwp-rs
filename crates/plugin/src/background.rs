//! The entrypoint for the background task where our actual VPN plugin runs.

use std::mem::ManuallyDrop;

use windows::{
    core::*,
    ApplicationModel::Background::IBackgroundTask,
    ApplicationModel::Background::IBackgroundTaskInstance,
    ApplicationModel::Background::IBackgroundTask_Impl,
    ApplicationModel::Core::CoreApplication,
    Networking::Vpn::{IVpnPlugIn, VpnChannel},
    Win32::Foundation::{E_INVALIDARG, E_NOINTERFACE, E_UNEXPECTED, S_OK},
    Win32::System::WinRT::IActivationFactory,
    Win32::System::WinRT::IActivationFactory_Impl,
};

/// The WinRT Activatable Class which acts as the entrypoint for the background tasks
/// which get invoked to handle the actual VPN tunnel.
#[implement(IBackgroundTask)]
pub struct VpnBackgroundTask;

impl IBackgroundTask_Impl for VpnBackgroundTask_Impl {
    fn Run(&self, task: Ref<IBackgroundTaskInstance>) -> Result<()> {
        let task = task.as_ref().ok_or(Error::from(E_UNEXPECTED))?;
        let deferral = task.GetDeferral()?;

        // Grab existing plugin instance from in-memory app properties or create a new one
        let app_props = CoreApplication::Properties()?;
        let plugin = if app_props.HasKey(&HSTRING::from("plugin"))? {
            app_props.Lookup(&HSTRING::from("plugin"))?.cast()?
        } else {
            let plugin: IVpnPlugIn = super::plugin::VpnPlugin::new().into();
            app_props.Insert(&HSTRING::from("plugin"), &plugin)?;
            plugin
        };

        // Call into VPN platform with the plugin object
        VpnChannel::ProcessEventAsync(&plugin, &task.TriggerDetails()?)?;

        deferral.Complete()?;

        Ok(())
    }
}

/// A factory object to generate `VpnBackgroundTask`.
///
/// Returned by `DllGetActivationFactory` when the system attempts to get an
/// instance of `VpnBackgroundTask`.
#[implement(IActivationFactory)]
struct VpnBackgroundTaskFactory;

impl IActivationFactory_Impl for VpnBackgroundTaskFactory_Impl {
    /// Creates and returns a new instance of `VpnBackgroundTask`.
    fn ActivateInstance(&self) -> Result<IInspectable> {
        Ok(VpnBackgroundTask.into())
    }
}

/// Called by any consumers of this library attempting to get instances of any activatable
/// Windows Runtime classes we support.
///
/// When the system is ready to launch our VPN background task, it needs to get a reference
/// to our `VpnBackgroundTask` object. It can do so because as part of our `AppxManifest.xml`
/// we list out which Activatable Classes (VpnBackgroundTask) we want registered during App
/// installation. Furthermore, we specify that the component is hosted in our DLL. From there,
/// it knows to query us via the `DllGetActivationFactory` function we export to get some
/// object implementing `IActivationFactory` which knows how to create new instances of the
/// target WinRT runtime class.
///
/// Since `activatable_class_id` is an _In_ parameter, the caller is responsible for freeing it.
/// But the HSTRING wrapper from the windows crate has a `Drop` impl which will attempt to free
/// it once it goes out of scope. Unfortunately, that would be a double-free once we've returned
/// back to the caller who would also attempt to free it. Hence, we transparently wrap the HSTRING
/// with ManuallyDrop to skip any free'ing on the Rust side.
#[no_mangle]
pub unsafe extern "system" fn DllGetActivationFactory(
    activatable_class_id: ManuallyDrop<HSTRING>,
    factory: *mut Option<IActivationFactory>,
) -> HRESULT {
    if activatable_class_id.is_empty() || factory.is_null() {
        return E_INVALIDARG;
    }

    // Return the appropriate factory based on which class was requested
    if *activatable_class_id == "WireGuard-UWP.VpnBackgroundTask" {
        *factory = Some(VpnBackgroundTaskFactory.into());
    } else {
        *factory = None;
        return E_NOINTERFACE;
    }

    S_OK
}
