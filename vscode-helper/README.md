# RepoJump Startup

This companion extension is bundled with RepoJump and installed from a local VSIX when Git Graph startup content is first used. Git Graph (`mhutchie.git-graph`) must already be installed and enabled.

It reads only one-time requests carried by workspaces created by RepoJump, checks the project and expiration, and runs `git-graph.view` for that project's root. Opening an ordinary folder or restoring a consumed workspace does not run a command. It does not install Git Graph or change project files, editor preferences or workspace trust.

Errors are reported to RepoJump. The project window remains open. Startup requests expire after 15 seconds.
