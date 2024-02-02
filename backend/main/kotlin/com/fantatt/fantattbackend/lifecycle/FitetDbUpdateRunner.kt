package com.fantatt.fantattbackend.lifecycle

import org.springframework.scheduling.annotation.Scheduled


val UPDATE_CMD = "python -m fitet --update --dump-path ${SchedulingConfig().dumpPhat}"

// one time per hour
@Scheduled(cron = "0 0 * * * *")
fun updateDb() {
    // TODO some locking mechanism
    Runtime.getRuntime().exec(UPDATE_CMD)
}