# User guide

1. Start the executable. Click a piece, then a highlighted destination. White and black pieces carry `W`/`B` prefixes so color is readable even without relying on board color.
2. Choose **Two players**, one of four computer policies, or **Local UCI engine**. In computer mode you play White. For Local UCI, enter the full path to a trusted engine executable in **Engine executable path**. Choose **Computer vs computer** and enable **Autoplay** to watch a match between internal policies.
3. For a timed game, choose minutes and **Clock mode**, then **New timed game**. Fischer adds the selected increment after each move. Bronstein gives each player a fresh delay before their main time starts falling. Hourglass transfers spent time to the other player's clock. **Pause clock** stops both the timer and move input. After restart the saved clock is paused; select **Resume clock**. Undo/redo is available only in untimed games.
4. Choose a promotion piece before moving a pawn to the last rank. Use **Undo**, **Redo**, **New game**, **Flip board**, and the theme toggle in the toolbar.
5. Use **Load FEN** for a legal standard position. **Copy FEN** and **Copy PGN** put text on the clipboard. **Load PGN** accepts one plain mainline; comments, variations and NAGs are rejected with an explanation.
6. When available, **Claim draw** handles threefold repetition or the 50-move rule. Checkmate, stalemate, insufficient material, fivefold repetition and 75-move draws end automatically.
7. The current game and clock autosave and reload on startup. See `offline-mode.md` for the save path.
8. To analyze with a local UCI engine, enter its full executable path in **Local UCI analysis** and click **Analyze**. The same path is used for UCI opponent play. No engine is included.
