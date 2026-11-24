//! javax.wireless.messaging through a Java program
//! (tests/io_java/WmaTest.java): messages are built and counted as on a
//! phone, and every send fails with `IOException`. The network is off:
//! messaging does not depend on it.

#[path = "io_support/mod.rs"]
mod io_support;

use fastemu_core::NetworkPolicy;

#[test]
fn messages_fail_cleanly_and_are_never_sent() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let output = io_support::run(classes, "WmaTest", &[], NetworkPolicy::Deny);
    let expected = "address sms://+15551234567 timestamp null text null
segments 1 1 2 1 2 0
send -> java.io.IOException: sending messages is not supported
receive -> a client connection cannot receive messages
listener -> a client connection cannot receive messages
binary sms://+1555:5000 200 segments 2
type -> unknown message type mms
address -> invalid message address \"http://example.com/\"
unaddressed -> the message has no address
null -> NullPointerException
closed -> connection is closed
port -> sms port 5000 is already in use
receive on close -> the connection was closed during receive
reopened
cbs -> cell broadcast connections cannot send messages
cbs client -> cell broadcast connections only receive: cbs://+123
empty -> sms:// has neither a number nor a port
done";
    io_support::assert_output(&output, expected);
}
