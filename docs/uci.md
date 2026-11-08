# UCI protocol subset

`chess-chaos-uci` implements one-shot standard-chess analysis, also used to select a move when playing White against a local engine. Handshake timeout: five seconds for `uciok` and five seconds for `readyok`. Search timeout: requested move time plus five seconds. Parsed `info`: depth, centipawn score, nodes, PV. It requires a nonempty `bestmove`; the chess core rejects illegal moves. There is no MultiPV, option negotiation, ponder, engine reuse, score bound interpretation, or crash recovery yet.

