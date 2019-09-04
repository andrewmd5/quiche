# Updater
- Function for creating a release package (zip)
- Function for running the installer creator?
- CLI for automating the creation of a new release which means creating a new Releases.toml, the update manifest, and all the above functions.
- Add function for setting/getting the Rainway version on the registry (we should update the one created by the installer)
- Add function for checking for updates. 
- Make auto updating safe by using a rollback system:
 - Write update package to a seperate folder. If this fails, the update stops.
 - Backup currently installed version to a seperate folder. If this fails, the update stops.
 - Try deleting all files in current installation. If any files fail to delete, the update fails, and we rollback changes.
 - If all files are deleted, move the update to the final destination. 
 - validate that all files are present 
- Add function for updating/installing to the latest version.
- Create final Javascript API for passing progress.
- Add commandline arguments that make it easy to interface with the process from C#.
- Self-updating functionality for the actual bootstrapper 


![](https://i.imgur.com/Hkm3NIg.png)