package com.fantatt.fantattbackend.db.matches

import com.fantatt.fantattbackend.lifecycle.SchedulingConfig
import org.springframework.stereotype.Component
import java.sql.Date
import java.sql.DriverManager
import java.sql.ResultSet
import java.time.LocalDateTime
import java.util.concurrent.locks.ReentrantReadWriteLock
import kotlin.concurrent.withLock
import kotlin.contracts.InvocationKind
import kotlin.contracts.contract
import kotlin.io.path.Path
import kotlin.io.path.absolutePathString

const val SELECT_MATCHES_BY_PLAYER_BETWEEN_DATES = """
    SELECT * FROM match
    INNER JOIN player p1 ON one_id = p1.id
    INNER JOIN player p2 ON two_id = p2.id
    INNER JOIN main.event e on e.id = match.event_id
    WHERE (p1.name = ? OR p2.name = ?)
    AND date BETWEEN ? AND ?
"""

@Component
class SqliteFitetParsedMatchRepository(
    schedulingConfig: SchedulingConfig
): MatchRepository {
    private val lock = ReentrantReadWriteLock()
    private val url = "jdbc:sqlite:${Path(schedulingConfig.dumpPhat).absolutePathString()}"

    override fun findAllMatchesByPlayerBetweenDates(
        player: String,
        startDate: LocalDateTime,
        endDate: LocalDateTime
    ): List<RealMatch> = lock.readLock().withLock {
            // not the best, I should use the spring boot configuration for the database,
            // but I need to change a lot of the default configuration, and I don't need
            // all that performance for this connection (it is used only sometimes, one
            // time per player per week)
            DriverManager.getConnection(url).use { conn ->
                 conn.prepareStatement(SELECT_MATCHES_BY_PLAYER_BETWEEN_DATES).use stmt@{ stmt ->
                    stmt.setString(1, player)
                    stmt.setString(2, player)
                    val date = Date.valueOf(startDate.toLocalDate())
                    stmt.setDate(3, date)
                    stmt.setDate(4, date)

                    return@stmt parseResultSet(stmt.executeQuery())
                }
            }
        }

    private fun parseResultSet(rs: ResultSet) = generateSequence {
        if (rs.next()) {
            RealMatch(
                Pair(rs.getString("p1.name"), rs.getString("p2.name")),
                parseSetString(rs.getString("_score")),
            )
        } else {
            null
        }
    }.toList()

    private fun parseSetString(setString: String): List<Pair<Int, Int>> {
        // maybe there is a better way to do this, but I'm lazy and it's enough
        try {
            // sets in python list format: [(1, 2), (3, 4), ...]
            assert(setString.first() == '[')
            assert(setString.last() == ']')
            return setString
                // drop the '[(...)]'
                .drop(2)
                .dropLast(2)
                .split("), (")
                .map {
                    val (a, b) = it.split(", ")
                    Pair(a.toInt(), b.toInt())
                }
        } catch (e: Exception) {
            throw IllegalArgumentException("Invalid set string format:", e)
        }
    }

    fun startUpdate() {
        lock.writeLock().lock()
    }

    fun endUpdate() {
        lock.writeLock().unlock()
    }

    final inline fun <T> update(block: () -> T): T {
        startUpdate()
        return try {
            block()
        } finally {
            endUpdate()
        }
    }

}