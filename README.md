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

You should always build with a target of `i686-pc-windows-msvc` and to produce a small binary for production use `cargo build --target=i686-pc-windows-msvc --release`. While Rainway only supports x64 systems, we need to be able to present errors on x32 host which is why we targeted i686.