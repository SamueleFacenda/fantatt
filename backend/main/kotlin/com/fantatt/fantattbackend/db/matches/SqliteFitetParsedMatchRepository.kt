package com.fantatt.fantattbackend.db.matches

class SqliteFitetParsedMatchRepository: MatchRepository {
    override fun findAllMatchesByPlayerBetweenDates(
        player: String,
        startDate: String,
        endDate: String
    ): List<RealMatch> {
        TODO("Not yet implemented")
    }

}