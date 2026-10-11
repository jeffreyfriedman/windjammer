# Windjammer for Vim and Neovim

Runtime files for `.wj`: file type detection (`ftdetect/windjammer.vim`), filetype settings (`ftplugin/windjammer.vim`) and
syntax highlighting (`syntax/windjammer.vim`).

Install by putting this directory on your `runtimepath`, for example with vim-plug:

```vim
Plug 'jeffreyfriedman/windjammer', {'rtp': 'editors/vim'}
```

For code intelligence, point your LSP client at the `windjammer-lsp` binary (see [crates/windjammer-lsp](../../crates/windjammer-lsp/README.md)).
