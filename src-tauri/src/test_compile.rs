use ssh2::Channel;

fn main() {
    // This is just to check if the method exists
    let mut channel: Channel = unsafe { std::mem::zeroed() };
    channel.request_window_change(80, 24, 0, 0);
}
