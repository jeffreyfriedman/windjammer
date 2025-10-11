" Vim syntax file
" Language: Windjammer
" Maintainer: Windjammer Contributors
" Latest Revision: 2025-01-01

if exists("b:current_syntax")
  finish
endif

" Keywords
syn keyword windjammerKeyword fn let mut const static struct enum trait impl type pub use as where unsafe dyn bound
syn keyword windjammerConditional if else match
syn keyword windjammerRepeat for while loop
syn keyword windjammerKeyword go defer await async
syn keyword windjammerKeyword break continue return
syn keyword windjammerSelf Self self

" Types
syn keyword windjammerType int int32 uint float bool string
syn match windjammerType "\<[A-Z][a-zA-Z0-9_]*\>"
syn keyword windjammerType Option Result Vec HashMap HashSet

" Booleans
syn keyword windjammerBoolean true false

" Decorators
syn match windjammerDecorator "@\w\+"

" Comments
syn keyword windjammerTodo contained TODO FIXME XXX NOTE
syn match windjammerLineComment "//.*$" contains=windjammerTodo
syn region windjammerBlockComment start="/\*" end="\*/" contains=windjammerTodo

" Strings
syn region windjammerString start=+"+ skip=+\\\\\|\\"+  end=+"+ contains=windjammerInterpolation
syn region windjammerInterpolation start="${" end="}" contained

" Characters
syn match windjammerChar "'\([^'\\]\|\\[nrt\\0'\"]\)'"

" Numbers
syn match windjammerNumber "\<\d\+\>"
syn match windjammerFloat "\<\d\+\.\d\+\([eE][+-]\?\d\+\)\?\>"

" Operators
syn match windjammerOperator "[-+*/%=<>!&|]"
syn match windjammerOperator "->"
syn match windjammerOperator "=>"
syn match windjammerOperator "<-"
syn match windjammerOperator "|>"
syn match windjammerOperator "\.\."
syn match windjammerOperator "\.\.\="
syn match windjammerOperator "?"
syn match windjammerOperator "::"

" Macros
syn match windjammerMacro "\w\+!"

" Functions
syn match windjammerFunction "\<\w\+\ze\s*("

" Highlighting
hi def link windjammerKeyword Keyword
hi def link windjammerConditional Conditional
hi def link windjammerRepeat Repeat
hi def link windjammerType Type
hi def link windjammerBoolean Boolean
hi def link windjammerDecorator PreProc
hi def link windjammerTodo Todo
hi def link windjammerLineComment Comment
hi def link windjammerBlockComment Comment
hi def link windjammerString String
hi def link windjammerInterpolation Special
hi def link windjammerChar Character
hi def link windjammerNumber Number
hi def link windjammerFloat Float
hi def link windjammerOperator Operator
hi def link windjammerMacro Macro
hi def link windjammerFunction Function
hi def link windjammerSelf Keyword

let b:current_syntax = "windjammer"

