use crate::error::{SubscriberErrorCode, ToResult};
use crate::qos::QoSProfile;
use crate::Node;
use crate::{rcl_bindings::*, RclReturnCode};
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::borrow::Borrow;
use cstr_core::CString;
use rosidl_runtime_rs::{Message, RmwMessage};

use parking_lot::{Mutex, MutexGuard};

pub struct SubscriptionHandle {
    handle: Mutex<rcl_subscription_t>,
    node_handle: Arc<Mutex<rcl_node_t>>,
}

impl SubscriptionHandle {
    pub fn lock(&self) -> MutexGuard<rcl_subscription_t> {
        self.handle.lock()
    }
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        let handle = self.handle.get_mut();
        let node_handle = &mut *self.node_handle.lock();
        // SAFETY: No preconditions for this function (besides the arguments being valid).
        unsafe {
            rcl_subscription_fini(handle as *mut _, node_handle as *mut _);
        }
    }
}

/// Trait to be implemented by concrete Subscriber structs
/// See [`Subscription<T>`] for an example
pub trait SubscriptionBase {
    fn handle(&self) -> &SubscriptionHandle;
    fn execute(&self) -> Result<(), RclReturnCode>;
}

/// Main class responsible for subscribing to topics and receiving data over IPC in ROS
pub struct Subscription<T>
where
    T: Message,
{
    pub handle: Arc<SubscriptionHandle>,
    // The callback's lifetime should last as long as we need it to
    pub callback: Mutex<Box<dyn FnMut(T) + 'static>>,
}

impl<T> Subscription<T>
where
    T: Message,
{
    pub fn new<F>(
        node: &Node,
        topic: &str,
        qos: QoSProfile,
        callback: F,
    ) -> Result<Self, RclReturnCode>
    where
        T: Message,
        F: FnMut(T) + Sized + 'static,
    {
        // SAFETY: Getting a zero-initialized value is always safe.
        let mut subscription_handle = unsafe { rcl_get_zero_initialized_subscription() };
        let type_support =
            <T as Message>::RmwMsg::get_type_support() as *const rosidl_message_type_support_t;
        let topic_c_string = CString::new(topic).unwrap();
        let node_handle = &mut *node.handle.lock();

        // SAFETY: No preconditions for this function.
        let mut subscription_options = unsafe { rcl_subscription_get_default_options() };
        subscription_options.qos = qos.into();
        unsafe {
            // SAFETY: The subscription handle is zero-initialized as expected by this function.
            // The node handle is kept alive because it is co-owned by the subscription.
            // The topic name and the options are copied by this function, so they can be dropped
            // afterwards.
            // TODO: type support?
            rcl_subscription_init(
                &mut subscription_handle as *mut _,
                node_handle as *mut _,
                type_support,
                topic_c_string.as_ptr(),
                &subscription_options as *const _,
            )
            .ok()?;
        }

        let handle = Arc::new(SubscriptionHandle {
            handle: Mutex::new(subscription_handle),
            node_handle: node.handle.clone(),
        });

        Ok(Self {
            handle,
            callback: Mutex::new(Box::new(callback)),
        })
    }

    /// Ask RMW for the data
    ///
    /// +-------------+
    /// | rclrs::take |
    /// +------+------+
    ///        |
    ///        |
    /// +------v------+
    /// |  rcl_take   |
    /// +------+------+
    ///        |
    ///        |
    /// +------v------+
    /// |  rmw_take   |
    /// +-------------+
    pub fn take(&self) -> Result<T, RclReturnCode> {
        let mut rmw_message = <T as Message>::RmwMsg::default();
        let handle = &mut *self.handle.lock();
        let ret = unsafe {
            // SAFETY: The first two pointers are valid/initialized, and do not need to be valid
            // beyond the function call.
            // The latter two pointers are explicitly allowed to be NULL.
            rcl_take(
                handle as *const _,
                &mut rmw_message as *mut <T as Message>::RmwMsg as *mut _,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
            )
        };
        ret.ok()?;
        Ok(T::from_rmw_message(rmw_message))
    }
}

impl<T> SubscriptionBase for Subscription<T>
where
    T: Message,
{
    fn handle(&self) -> &SubscriptionHandle {
        self.handle.borrow()
    }

    fn execute(&self) -> Result<(), RclReturnCode> {
        let msg = match self.take() {
            Ok(msg) => msg,
            Err(RclReturnCode::SubscriberError(SubscriberErrorCode::SubscriptionTakeFailed)) => {
                // Spurious wakeup – this may happen even when a waitset indicated that this
                // subscription was ready, so it shouldn't be an error.
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        (*self.callback.lock())(msg);
        Ok(())
    }
}
