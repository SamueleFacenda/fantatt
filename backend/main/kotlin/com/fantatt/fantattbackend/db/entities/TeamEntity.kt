package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.Entity
import jakarta.persistence.GeneratedValue
import jakarta.persistence.Id
import jakarta.persistence.ManyToOne

@Entity
data class TeamEntity(
    var score: Int = 0,
    val name: String,
    // IMPORTANT: division is 1-indexed
    val division: Int,
    @ManyToOne
    val society: SocietyEntity,
    @Id @GeneratedValue val id: Long?=null
)