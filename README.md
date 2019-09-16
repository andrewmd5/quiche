# What is this?

For our installation process to be smooth, we need to validate that a users system meets some basic requirements. This bootstrapper will quickly scan a user's PC to do just that. if all is well, it will download the primary installer and launch it. All in all it only takes a few seconds. 

![](https://i.imgur.com/o8ayR5b.gif)


By using a bootstrapper we also:
- Increase user delight by obfuscating large initial downloads.
- Prevent AV false positives.
- Have a single file that is downloaded by everyone. So no more Chrome thinking our new installers are "dangerous."
- Can do any prechecks, error handling, or setup ahead of the actual installer.
- Can do personalized bootstrappers to auto login users.


# Why Rust?
- It produces incredibly tiny binaries. (< ~2 MB)
- It's safe. 
- It has no runtime. 
- It's fast. 
- I like Rust. 



# Building

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Run `rustup install stable-i686-pc-windows-msvc`
3. Pull this repo
4. Run `cargo build`

To produce a small binary for production use `cargo build --target=i686-pc-windows-msvc --release`. While Rainway only supports x64 systems, we need to be able to present errors on x32 host which is why we targeted i686.

# Running

1. Setup the dummy release server, see the README [here](/dummy_server/README.MD)
2. Run `cargo run` as Administrator

The UI is loaded from the [index.html](/resources/index.html) resource file. Please ensure all your assets are inlined and not remote HTTP calls are made, the UI should function offline and be small. 


# Callbacks

The Rust code will perform actions and post results back to hardcoded callbacks. These callbacks consist of a completion, which signals the task finished succesfully, or a failure which contains an error message. Downloading has an extra callback for progress.  

