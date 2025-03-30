pub trait MaybeSend {}

pub trait MaybeSendSync {}

impl<T> MaybeSendSync for T {}

impl<T> MaybeSend for T {}
