package com.fantatt.fantattbackend.db.matches

import org.springframework.stereotype.Component
import java.time.LocalDateTime

@Component
class SqliteFitetParsedMatchRepository: MatchRepository {
    override fun findAllMatchesByPlayerBetweenDates(
        player: String,
        startDate: LocalDateTime,
        endDate: LocalDateTime
    ): List<RealMatch> {
        TODO("Not yet implemented")
    }

}