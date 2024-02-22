package com.fantatt.fantattbackend.db.matches

import org.springframework.stereotype.Component
import java.time.LocalDateTime
import java.util.concurrent.Phaser
import java.util.concurrent.locks.ReentrantLock
import java.util.concurrent.locks.ReentrantReadWriteLock

@Component
class SqliteFitetParsedMatchRepository: MatchRepository {
    private val lock = ReentrantReadWriteLock()

    override fun findAllMatchesByPlayerBetweenDates(
        player: String,
        startDate: LocalDateTime,
        endDate: LocalDateTime
    ): List<RealMatch> {
        lock.readLock().lock()

        try {
            TODO()
        } finally {
            lock.readLock().unlock()
        }
    }

    fun startUpdate() {
        lock.writeLock().lock()
    }

    fun endUpdate() {
        lock.writeLock().unlock()
    }

}