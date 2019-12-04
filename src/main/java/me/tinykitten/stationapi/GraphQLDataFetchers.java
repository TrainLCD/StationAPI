package me.tinykitten.stationapi;

import com.google.common.collect.ImmutableMap;
import graphql.schema.DataFetcher;
import org.springframework.stereotype.Component;

import java.util.Arrays;
import java.util.List;
import java.util.Map;

@Component
public class GraphQLDataFetchers {
    private static List<Map<String, String>> stations = Arrays.asList(
            ImmutableMap.of("id", "1")
    );

    private static List<Map<String, String>> lines = Arrays.asList(
            ImmutableMap.of("id", "1")
    );

    public DataFetcher lineDataFetcher() {
        return dataFetchingEnvironment -> {
            String lineId = dataFetchingEnvironment.getArgument("id");
            return lines
                    .stream()
                    .filter(l -> l.get("id").equals(lineId))
                    .findFirst()
                    .orElse(null);
        };
    }
}
