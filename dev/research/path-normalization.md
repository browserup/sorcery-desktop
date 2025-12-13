# Path Normalization

Path normalization is our nickname for aligning paths to workspace paths and creating a new, real path to 
the user's file we will edit. There are various scenarios where a provided "source path" needs to be aligned
to a proper "target path"

A user maintains one or more "workspaces" which are mapped to a local path on-disk where their copy of a
repo exists.

In a number of development scenarios, the user is presented with a path fragment, or relative path.

The key is that the fragment may, or may not have a workspace as the first part of it's relative path.

If it *does* have a valid workspace in the begingging of the path, then we are in good shape. We check only that
workspace for the path segments presented. If it exists, we open it. If not, we do nothing.

Note that there is a pre-step when a "rev" is passed. We should attempt to check out the "rev" first. Then, if the
file is there, we open it.

# Full Path Alignment (to fragment relative to workspace)

If the user has the setting "Allow opening files outside of configured workspaces" enabled, we will try and 
see if the file exists, and open it.  If it exactly matches a path in a workspace, that is ok too.

If the setting is not set, we do nothing with full paths *unless* they exactly point within a workpace.

## Path fragment (alignment to most-recent matching workspace)

In other cases, we might get a path fragment. This might be as simple as "package.json" or deeper, like

app/models/account.json:53

or 

./spec/models/callbacks/black_knight_order_validation_spec.rb

Note that I usually call it a fragment path although it is a relative path.

* We will allow ./spec/models/callbacks/black_knight_order_validation_spec.rb, as we will rewrite it to
  spec/models/callbacks/black_knight_order_validation_spec.rb before joining it. We don't allow any 
 relative directory/path commands in our final paths. For example ../spec/models/callbacks/black_knight_order_validation_spec.rb
would be problematic. 

We fully resolve intended paths before we test them. Then we check if they match a viable file that is in a workspace
(unless the )

hat's somewhat true, as we don't allow .. and directory climbing, for security reasons.

In order to match path fragments, we need to look in the workspaces for a matching subpaths to apps/model/user.rb

It is possible there could be more than one match. After all, user.rb is a pretty common model name.
The answer, for our approach, is that we return the first match, but we are careful about the order we look in.
We look in most recently active workspace first, so we have the highest chances of giving the user 
the file they are actually looking for. We check its existence, and open it if it exists where we expect.

The selection algorithm uses the dev/workspace-mru-spec.md algorithm to track which is most recently used.
Let's implement that algorithm now.

# Fragment Matching algorithm overview
Goal:  We want to match the fragment path we are given by combining the 

workpace path + subpath fragment

When we have the workspace, this is easy. If we don't then we try to find it in our mapped workspaces.

We will add a setting in our syntax for:

workspaceHint=

srcuri://README.md:50?workspaceHint=browserup

This is for when the workspace may or may not be a repo name. It isn't part of the path, as that would imply we
know it is a workspace. Instead, it is given as a hint--so it if matches a workspace name, we will try pre-pending
it to the path fragment, and check in that workspace first. Note: this check should be case insensitive. If it 
does not match a workspace, we should carry on with our normal other checks.

this use-case is for when we get a path from datadog, where we know the service name, but not the repo name. This
may help us find the path, but it isn't a guarantee that it lines up with the name.

we will check for a matching
file there first, and use that if we find one. We do this *before* checking the most recently used workspaces in-order.

