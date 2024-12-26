# wdk-mutex tests

This crate supports the wdk-mutex crate by implementing a number of tests as a driver which must pass to 
validate the [wdk-mutex](https://github.com/0xflux/wdk-mutex) crate.

## Usage

To run the tests, ensure you have the WDK setup as per Microsoft documentation, build the driver via `cargo make`, and install it in your
test environment.

Running the driver will produce debug messages (either [WinDbg](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/) 
or [DebugView](https://learn.microsoft.com/en-us/sysinternals/downloads/debugview)) as to whether the test passes or fails.

## Contributions 

This crate is in support of the main crate at [wdk-mutex](https://github.com/0xflux/wdk-mutex). Contributions and issues are welcome on this
repository, as well as in [wdk-mutex](https://github.com/0xflux/wdk-mutex).