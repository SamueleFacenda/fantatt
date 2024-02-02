package com.fantatt.fantattbackend.lifecycle

import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.context.annotation.Configuration
import org.springframework.scheduling.annotation.EnableScheduling

@Configuration
@EnableScheduling
@ConfigurationProperties(prefix = "fantatt.scheduling")
class SchedulingConfig {
    val dumpPhat: String = ""
}