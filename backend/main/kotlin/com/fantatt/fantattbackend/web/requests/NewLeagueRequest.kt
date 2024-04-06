package com.fantatt.fantattbackend.web.requests

import com.fantatt.fantattbackend.db.entities.SocietyEntity

data class NewLeagueRequest(
    val name: String,
    val teams: List<SocietyEntity>
)