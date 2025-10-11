" Filetype plugin for Windjammer
" Only do this when not done yet for this buffer
if exists("b:did_ftplugin")
  finish
endif
let b:did_ftplugin = 1

" Set comment strings
setlocal comments=s1:/*,mb:*,ex:*/,://
setlocal commentstring=//\ %s

" Set formatting options
setlocal formatoptions-=t formatoptions+=croql

" Set indentation
setlocal shiftwidth=4
setlocal tabstop=4
setlocal softtabstop=4
setlocal expandtab
setlocal autoindent
setlocal smartindent

" Enable folding
setlocal foldmethod=syntax
setlocal foldlevel=99

" Matchit support for keywords
if exists("loaded_matchit")
  let b:match_words = '\<if\>:\<else\>,\<match\>:\<=>\>,\<for\>:\<in\>,\<loop\>:\<break\>'
endif

" LSP Configuration
" This will be loaded if you have vim-lsp or nvim-lspconfig

