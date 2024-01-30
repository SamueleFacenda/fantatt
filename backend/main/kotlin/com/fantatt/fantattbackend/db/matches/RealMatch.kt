package com.fantatt.fantattbackend.db.matches

data class RealMatch(
    val players: Pair<String, String>,
    val sets: List<Pair<Int, Int>>
) {
    val setResults = sets.map { (oneScore, twoScore) -> if (oneScore > twoScore) 1 else -1 }
    val winner by lazy { if (setResults.sum() > 0) players.first else players.second }
}
