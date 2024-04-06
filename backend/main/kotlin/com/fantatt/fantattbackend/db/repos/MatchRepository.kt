package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.MatchEntity
import com.fantatt.fantattbackend.db.entities.MatchId
import org.springframework.data.repository.CrudRepository

interface MatchRepository: CrudRepository<MatchEntity, MatchId> {
}