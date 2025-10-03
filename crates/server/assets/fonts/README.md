# Font Setup for Toolbar Icons

To use Nerd Font icons in the toolbar, you need to download and place the font file here.

## Steps:

1. Go to https://www.nerdfonts.com/font-downloads
2. Download "FiraCode Nerd Font" (or any other Nerd Font)
3. Extract the font files
4. Copy `FiraCodeNerdFont-Regular.ttf` to this folder

The file structure should be:
```
assets/
└── fonts/
    ├── README.md (this file)
    └── FiraCodeNerdFont-Regular.ttf
```

## Alternative Fonts:
You can use any Nerd Font. Just update the filename in `src/plugins/toolbar.rs` in the `load_nerd_font` function.

Popular options:
- FiraCode Nerd Font
- JetBrains Mono Nerd Font
- Hack Nerd Font
- Source Code Pro Nerd Font

## Testing:
After placing the font file, run the application and check the console for:
- "Loading Nerd Font from fonts/FiraCodeNerdFont-Regular.ttf"
- "Setting up toolbar with Nerd Font"

If you see "Setting up toolbar without Nerd Font", the font file wasn't found.