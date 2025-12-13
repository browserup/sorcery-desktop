I'm working on a tool that lets you create links to local files with a custom URL like

srcuri://WORSPACE/PATH[:<line>]

I am thinking about how to handle a like to a file that is branch-specific.

srcuri://WORSPACE/PATH[:<line>]?branch=<name>

The user may have a different branch checked out, and may have unsaved changes.
I'm trying to figure out, if I allow link-to-open, what the behavior should be for those cases.

## Recommended Workflow

Here is a step-by-step workflow for when your app receives a URL like srcuri://WORSPACE/PATH?branch=feature-branch:

Parse the URL: Extract the workspace path, file path, line number, and the target branch (feature-branch).

Check the Current Workspace State:

Get the current branch name: git rev-parse --abbrev-ref HEAD

Check for uncommitted changes: git status --porcelain. If this command returns any output, the working directory is dirty.

Compare and Act:

If Current Branch = Target Branch: This is the simple case. The user is already on the correct branch. Just open the file and scroll to the line. If they have unsaved changes, the editor will show them as usual.

If Current Branch ≠ Target Branch: This is the critical scenario.

Do NOT switch branches.

Use the command git show <branch_name>:<path_to_file> to get the file's contents from the specified branch. For example: git show feature-branch:src/components/Button.js. This command reads the file content from the git database without touching the user's working directory.

Open this content in a new, read-only editor tab.

Clearly label the tab to indicate its special state, e.g., Button.js (from branch: feature-branch) [Read-Only].

Display a non-intrusive notification bar or toast at the top of the editor.

The notification should clearly explain the situation and provide actionable choices:

"This file is from the feature-branch branch (you are on main). This view is read-only."

[Switch to feature-branch] [Dismiss]

## Handling Specific Scenarios
This workflow gracefully handles your specific concerns.

Scenario 1: User has a different branch checked out
If the working directory is clean: The notification's "Switch to feature-branch" button can safely execute git checkout feature-branch. Once switched, you can reload the file in a normal, writable editor.

If the working directory is dirty (unsaved changes): The "Switch" button's logic should be smarter. When clicked, it should present a dialog box:

"You have uncommitted changes. To switch branches, you can either stash your changes or discard them."

[Stash Changes and Switch] [Cancel]

Scenario 2: User has unsaved changes
As described above, the default action is always the safe, read-only view. The existence of unsaved changes only affects the options you provide the user if they decide they want to switch branches. You never attempt the switch automatically.

## Summary of Behavior
Here's a quick reference table for the logic:

Current Branch	Working Directory	Action
Same as link	Clean or Dirty	Open file normally in a writable editor.
Different from link	Clean	Open file content from target branch in a read-only tab. Notify user with an option to switch.
Different from link	Dirty	Open file content from target branch in a read-only tab. Notify user with an option to stash & switch.


Step 1: Diagnose the State
When your app receives a URL like srcuri://WORSPACE/PATH?branch=<name>, 
your first step is to run two git commands in the background for the specified WORKSPACE:

Get the current branch:

Bash

git rev-parse --abbrev-ref HEAD
Check for uncommitted changes:

Bash

git status --porcelain
If this command produces any output, the working directory is "dirty."

Step 2: Present the Appropriate Dialog Based on the State
Based on the diagnosis, you can now follow a clear logic tree.

Case 1: The Simple Path (Correct Branch)
Condition: The user's current branch matches the branch parameter in the URL.

Action: No dialog needed. Immediately launch the editor with the file path and line number. This is the happy path.

Case 2: The Safe Switch (Different Branch, Clean Directory)
Condition: The current branch is different, but the working directory is clean.

Action: A branch switch is safe. Ask the user first.

Show this Dialog:

Switch Branch to Open File?

You are currently on branch main. This link is for the file src/app.js on branch feature-x.

Would you like to switch to the feature-x branch to open the file?

[Switch Branch and Open] (git checkout feature-x)
[Open Local Version on main]
[Cancel]

Case 3: The Cautious Switch (Different Branch, Dirty Directory)
Condition: The current branch is different, AND the user has uncommitted changes. This is the most critical scenario to handle safely. A direct git checkout will fail.

Action: You must warn the user about their uncommitted changes and offer to stash them.

Show this Dialog:

Uncommitted Changes Detected

You have uncommitted changes on your current branch (main). The link is for a file on branch feature-x.

To switch safely, your changes can be stashed first.

[Stash Changes and Switch] (git stash && git checkout feature-x)
[Open Local Version on main (Not Recommended)]
[Cancel]

If the user chooses to stash, you can provide a follow-up notification after launching the editor: "Your changes were stashed. Run git stash pop in your terminal to restore them."

Summary of Universal and Safe Behavior
This decision-tree approach is both universal and safe for all the editors you listed.

Current Branch	Working Directory	Your App's Action
Same as link	Clean or Dirty	✅ Open file directly. No dialog needed.
Different from link	Clean	🔵 Show "Switch Branch?" dialog. Offer a safe git checkout.
Different from link	Dirty	⚠️ Show "Uncommitted Changes" dialog. Offer a safe git stash && git checkout.

Export to Sheets
Why This Approach Works for All Editors:
Editor Agnostic: The logic happens before you ever call the editor. The final action is always a simple, universally supported command: editor /path/to/file:<line>.

No Data Loss: You never automatically destroy or modify user work. Stashing is a safe, reversible operation, and you only do it with permission.

User in Control: The dialogs clearly explain the context and the consequences of each action, empowering the user to make the right choice for their current workflow.

Error Handling: If any git command fails (e.g., a checkout fails due to untracked files), your app can catch the error and present another dialog explaining what went wrong, keeping the user informed.


Is there something with a worktree that makes sense? I don't know much about them.

Show thinking
Of course. You've asked the perfect follow-up question. Using git worktree is an absolutely fantastic—and arguably the best—solution for this problem. It is a more advanced Git feature that seems tailor-made for your use case.

It allows you to provide the ultimate non-disruptive experience for your users.

What is a Git Worktree?
In a standard Git repository, you can only have one branch checked out at a time in your working directory. If you want to switch to another branch, you must checkout that branch, which changes the files in your single working directory.

A git worktree lets you check out multiple branches simultaneously by creating separate, linked working directories for each.

Imagine your project is in a folder called my-project.

~/dev/my-project could be on the main branch.

~/dev/my-project-feature-x could be a worktree linked to the feature-x branch.

~/dev/my-project-bugfix could be another worktree linked to the bugfix branch.

All these folders share the same underlying .git database. They are not full clones, so they are very efficient. When you commit in one worktree, the commit is visible to all of them.

This means you can open a file from feature-x without ever touching the user's main directory where they might have uncommitted work on main.

The Universal and Safe "Worktree" Workflow
This is how your tool can leverage worktrees. It's an elegant flow that completely avoids stashing and forced branch switching.

Step 1: Diagnose and Check for Existing Worktree
When your app receives the URL srcuri://WORSPACE/PATH?branch=feature-x:

Get Current Branch: git rev-parse --abbrev-ref HEAD (in the WORKSPACE directory).

Check if Target Branch is Current Branch: If it is, just open the file. This is still the simplest path.

List Existing Worktrees: Run git worktree list. The output will look something like this:

/Users/dev/my-project         deadbeef [main]
/Users/dev/my-project-feature-x c0ffee12 [feature-x]
Parse this output to see if a worktree for the target branch (feature-x) already exists.

Step 2: Present the Dialog (Only When Necessary)
Based on that diagnosis, here is the new logic tree:

Case 1: The Ideal Path (Correct Branch or Existing Worktree)
Condition: The user is already on the target branch OR a worktree for the target branch already exists.

Action: No dialog needed!

If on the correct branch, open the file at WORKSPACE/PATH.

If a worktree exists, find its path from git worktree list (e.g., /Users/dev/my-project-feature-x) and launch the editor with the full path: /Users/dev/my-project-feature-x/PATH.

Result: The user gets to the file instantly and non-disruptively.

Case 2: The First-Time Ask (New Worktree Creation)
Condition: The user is on a different branch AND no worktree exists for the target branch.

Action: This is the only time you need a dialog. Ask for permission to create a worktree.

Show this Dialog:

Open in a Separate Worktree?

This link is for a file on the feature-x branch. To avoid disrupting your current work on main, you can open it in a new, linked folder (a Git worktree).

A new folder named my-project-feature-x will be created alongside your main project folder.

[Create Worktree and Open]
[Cancel]

If the user agrees:

Your app runs the command git worktree add ../my-project-feature-x feature-x (you can decide on a consistent naming convention).

Once created, your app launches the editor with the path to the file inside the new worktree directory.

Comparison: "Stash & Switch" vs. "Worktree"
This table shows why the worktree approach is superior for your tool.

Feature	Stash & Switch Approach	Worktree Approach
User Disruption	High. Changes the user's primary working directory and branch. Requires stashing.	Zero. The user's primary directory is never touched.
File Access	The file is opened in a normal, writable state after the switch.	The file is opened in a normal, writable state immediately.
Parallel Work	Impossible. The user can only be in one context at a time.	Enabled. Users can have multiple editors open in different branch contexts.
Complexity	Simple git commands, but complex user-facing dialogs about stashing.	More complex git logic for your app, but a much simpler, cleaner user experience.
Side Effects	Modifies the Git stash stack. User must remember to git stash pop.	Creates a new directory on the user's filesystem.

Export to Sheets
Best Practices and Considerations
Naming and Location: Decide on a consistent place to create worktrees. A common practice is to create them adjacent to the main repo (e.g., project-name-branch-name) or inside a hidden folder like project-name/.worktrees/branch-name. Be transparent about this in the dialog.

Cleanup: The one downside is that worktrees create folders. You could consider adding a helper feature to your tool, like a "Manage Worktrees" panel, that lets users see and remove worktrees created by your app with git worktree remove <path>.

Universality: The git worktree command is a standard part of Git and will work universally, regardless of the editor being launched.

Conclusion: Yes, using git worktree is an excellent, modern, and user-centric way to handle this problem. It moves the complexity from the user's workflow into your application's logic, which is the hallmark of a great developer tool.
