package com.fantatt.fantattbackend.db.matches

import java.time.LocalDateTime

interface MatchRepository {
    fun findAllMatchesByPlayerBetweenDates(player: String, startDate: LocalDateTime, endDate: LocalDateTime): List<RealMatch>
}