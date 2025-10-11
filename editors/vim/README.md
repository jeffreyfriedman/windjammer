# Windjammer for Vim/Neovim

Syntax highlighting and LSP support for the Windjammer programming language in Vim and Neovim.

## Features

- 🎨 **Syntax Highlighting** - Full syntax highlighting for `.wj` files
- 🧠 **LSP Integration** - Connect to `windjammer-lsp` for code intelligence
- 📝 **Smart Indentation** - Automatic indentation and formatting
- 💬 **Comment Support** - Line and block comments
- 🔍 **Matchit Support** - Jump between matching keywords

## Installation

### Using vim-plug

Add to your `.vimrc` or `init.vim`:

```vim
Plug 'jeffreyfriedman/windjammer', {'rtp': 'editors/vim'}
```

Then run `:PlugInstall`

### Using packer.nvim (Neovim)

Add to your `init.lua`:

```lua
use {
  'jeffreyfriedman/windjammer',
  rtp = 'editors/vim'
}
```

### Manual Installation

Copy the files to your Vim configuration directory:

```bash
# For Vim
cp -r editors/vim/* ~/.vim/

# For Neovim
cp -r editors/vim/* ~/.config/nvim/
```

## LSP Setup

### Neovim with nvim-lspconfig

Install `nvim-lspconfig`, then add to your `init.lua`:

```lua
-- Windjammer LSP configuration
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register Windjammer LSP
if not configs.windjammer_lsp then
  configs.windjammer_lsp = {
    default_config = {
      cmd = {'windjammer-lsp'},
      filetypes = {'windjammer'},
      root_dir = function(fname)
        return lspconfig.util.find_git_ancestor(fname) or vim.fn.getcwd()
      end,
      settings = {},
    },
  }
end

-- Setup Windjammer LSP
lspconfig.windjammer_lsp.setup{
  capabilities = require('cmp_nvim_lsp').default_capabilities(),
  on_attach = function(client, bufnr)
    -- Enable completion
    vim.api.nvim_buf_set_option(bufnr, 'omnifunc', 'v:lua.vim.lsp.omnifunc')
    
    -- Key mappings
    local opts = { noremap=true, silent=true, buffer=bufnr }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
    
    -- Enable inlay hints for ownership inference
    if client.server_capabilities.inlayHintProvider then
      vim.lsp.inlay_hint.enable(bufnr, true)
    end
  end,
}

-- Auto-format on save (optional)
vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = "*.wj",
  callback = function()
    vim.lsp.buf.format({ async = false })
  end,
})
```

### Vim with vim-lsp

Install `vim-lsp`, then add to your `.vimrc`:

```vim
" Register Windjammer LSP
if executable('windjammer-lsp')
  au User lsp_setup call lsp#register_server({
    \ 'name': 'windjammer-lsp',
    \ 'cmd': {server_info->['windjammer-lsp']},
    \ 'whitelist': ['windjammer'],
    \ })
endif

" LSP key mappings
function! s:on_lsp_buffer_enabled() abort
  setlocal omnifunc=lsp#complete
  nmap <buffer> gd <plug>(lsp-definition)
  nmap <buffer> gr <plug>(lsp-references)
  nmap <buffer> K <plug>(lsp-hover)
  nmap <buffer> <leader>rn <plug>(lsp-rename)
  nmap <buffer> <leader>ca <plug>(lsp-code-action)
endfunction

augroup lsp_install
  au!
  autocmd User lsp_buffer_enabled call s:on_lsp_buffer_enabled()
augroup END
```

### CoC.nvim (Neovim/Vim)

Add to your `coc-settings.json`:

```json
{
  "languageserver": {
    "windjammer": {
      "command": "windjammer-lsp",
      "filetypes": ["windjammer"],
      "rootPatterns": [".git/", "."],
      "settings": {}
    }
  }
}
```

## Usage

Once installed, open any `.wj` file and you'll get:

- ✅ Syntax highlighting
- ✅ LSP features (if configured):
  - Auto-completion (`<C-x><C-o>` in Vim, automatic in Neovim with nvim-cmp)
  - Go to definition (`gd`)
  - Find references (`gr`)
  - Hover information (`K`)
  - Rename symbol (`<leader>rn`)
  - **Ownership inference hints** ✨ (shows inferred `&`, `&mut`, `owned`)

## Key Mappings

The following key mappings are set up when LSP is active:

| Key | Action |
|-----|--------|
| `gd` | Go to definition |
| `gr` | Find references |
| `K` | Show hover information |
| `<leader>rn` | Rename symbol |
| `<leader>ca` | Code actions |

## Configuration

### Customize Syntax Highlighting

Add to your `.vimrc` or `init.vim`:

```vim
" Custom Windjammer syntax colors
hi windjammerDecorator guifg=#FFA500 ctermfg=214
hi windjammerMacro guifg=#FF69B4 ctermfg=205
```

### Disable Inlay Hints

If you don't want ownership inference hints:

```lua
-- Neovim
vim.lsp.inlay_hint.enable(bufnr, false)
```

```vim
" Vim with vim-lsp
let g:lsp_inlay_hints_enabled = 0
```

## Requirements

- Vim 8.0+ or Neovim 0.5+
- `windjammer-lsp` binary in PATH:
  ```bash
  cargo install windjammer
  ```

## Troubleshooting

### LSP not starting

Check that `windjammer-lsp` is in your PATH:

```bash
which windjammer-lsp
```

View LSP logs (Neovim):

```vim
:LspInfo
:LspLog
```

### Syntax highlighting not working

Ensure filetype detection is enabled:

```vim
:filetype detect
:set filetype?
```

Should show: `filetype=windjammer`

## Contributing

Found a bug or have a feature request? [Open an issue](https://github.com/jeffreyfriedman/windjammer/issues)!

## License

MIT OR Apache-2.0

