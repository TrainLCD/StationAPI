package me.tinykitten.stationapi;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import software.amazon.codeguruprofilerjavaagent.Profiler;

@SpringBootApplication
public class StationapiApplication {

	public static void main(String[] args) {
		new Profiler.Builder()
				.profilingGroupName("StationAPI")
				.build().start();
		SpringApplication.run(StationapiApplication.class, args);
	}

}
