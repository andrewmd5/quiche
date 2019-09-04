# Updater
- Function to create a catalog of all files in a build. (file_name, sha_256)
- Function for creating the file changeset for the new and current releases. (should leverage the current versions catalog) and return new/changed, and deleted files. 
- Function for creating a release package (zip)
- Function for running the installer creator?
- CLI for automating the creation of a new release which means creating a new Releases.toml, the update manifest, and all the above functions.
- Add function for setting/getting the Rainway version on the registry (we should update the one created by the installer)
- Add function for checking for updates. 
- Add function for updating to the latest version (which should mean if someone is on version 1.0.5 but the latest is 1.0.8 we download & apply all the version inbetween)
- Design progress reporting. Rather than report the progress of downloads, it probably makes sense to report the progress of remaining actions. We may have multiple updates to apply, and each update only has a few phases (Download, Verify, Apply, Done.)
- Create a sane way to send progress reports to the webview UI without blocking.
- Add commandline arguments that make it easy to interface with the process from C#.
- Self-updating functionality for the actual bootstrapper 


![](https://i.imgur.com/Hkm3NIg.png)