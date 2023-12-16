package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.sql.Date

@Entity
class Round (
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    @ManyToOne
    val season: Season,
    val startDate: Date,
    val index: Int
)