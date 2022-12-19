# Concordium @ SheBuilds Workshop

## dApp instructions

1. Open the `dapp/` folder in a terminal.
2. Run `npm install` to install the dependencies.
3. Run the app with `npm start`.

For using the dapp, you need the ![Concordium Wallet browser extension](https://chrome.google.com/webstore/detail/concordium-wallet/mnnkpffndmickbiakofclnpoiajlegmg).

### Using `yarn` (on unix systems)

Some of the node modules we use have Windows-type line endings (`\r\n`), instead
of unix line endings (`\n`), which causes problems when using the `yarn` package
manager.

If you see an error message similar to this, then you've run into the problem:

``` sh
env: node\r: No such file or directory
```

The issue does not occur with `npm` because it automatically fixes the line
endings on unix systems.
However, it is possible to use `yarn`, but you need to fix the line endings
before it will work.
This guide explains how to fix the line endings on macOS: https://techtalkbook.com/env-noder-no-such-file-or-directory/
