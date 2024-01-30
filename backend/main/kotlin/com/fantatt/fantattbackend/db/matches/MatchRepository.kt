package com.fantatt.fantattbackend.db.matches

interface MatchRepository {
    fun findAllMatchesByPlayerBetweenDates(player: String, startDate: String, endDate: String): List<RealMatch>
}