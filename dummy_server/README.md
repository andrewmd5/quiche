- Extract Dummy.zip into this folder.
- Start an HTTP server on port 8080 which uses the folder as root. If you've got node installed, you can execute `yarn && yarn run dev` in this dir.
- Verify you can view http://local.vg:8080/Releases.toml
- Launch the Bootstrapper and it should now check if Rainway is installed.
  - If it is, it will download the update from /2.0.0/packages.zip
  - If it is not, it will download and run the current Rainway installer.