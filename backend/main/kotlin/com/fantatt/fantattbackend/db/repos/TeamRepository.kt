package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Team
import org.springframework.data.repository.CrudRepository

interface TeamRepository: CrudRepository<Team, Long> {
}