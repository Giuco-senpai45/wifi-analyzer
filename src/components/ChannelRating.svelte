<script lang="ts">
    import { onMount } from "svelte";
    import * as d3 from "d3";

    // Define our props
    export let channelData: { channel: number; occupancy: number }[];

    // Create a reactive statement for sorted data
    $: normalizedData = Array.from({ length: 13 }, (_, i) => {
        const existing = channelData.find((d) => d.channel === i + 1);
        return existing || { channel: i + 1, occupancy: 0 };
    });

    // State for sorting
    let sortBy = "channel";

    // Reactive sorting
    $: sortedData = [...normalizedData].sort((a, b) => {
        if (sortBy === "channel") return a.channel - b.channel;
        return b.occupancy - a.occupancy;
    });

    // Helper functions
    const formatOccupancy = (value: number): string =>
        `${(value * 100).toFixed(1)}%`;

    const getChannelQuality = (occupancy: number): string => {
        if (occupancy < 0.3) return "Excellent";
        if (occupancy < 0.5) return "Good";
        if (occupancy < 0.7) return "Fair";
        return "Poor";
    };

    const getChannelColor = (occupancy: number): string => {
        if (occupancy < 0.3) return "text-green-500";
        if (occupancy < 0.5) return "text-blue-500";
        if (occupancy < 0.7) return "text-yellow-500";
        return "text-red-500";
    };

    // D3 Chart setup
    let chartContainer: HTMLDivElement;

    const createChart = () => {
        // Clear existing content
        d3.select(chartContainer).selectAll("*").remove();

        const margin = { top: 20, right: 30, bottom: 40, left: 60 };
        const width = chartContainer.clientWidth - margin.left - margin.right;
        const height = 300 - margin.top - margin.bottom;

        const svg = d3
            .select(chartContainer)
            .append("svg")
            .attr("width", width + margin.left + margin.right)
            .attr("height", height + margin.top + margin.bottom)
            .append("g")
            .attr("transform", `translate(${margin.left},${margin.top})`);

        // Create scales
        const x = d3
            .scaleBand()
            .domain(normalizedData.map((d) => d.channel.toString()))
            .range([0, width])
            .padding(0.1);

        const y = d3.scaleLinear().domain([0, 1]).range([height, 0]);

        // Add axes
        svg.append("g")
            .attr("transform", `translate(0,${height})`)
            .call(d3.axisBottom(x));

        svg.append("g").call(
            d3.axisLeft(y).tickFormat((d) => formatOccupancy(d)),
        );

        // Add bars
        svg.selectAll("rect")
            .data(normalizedData)
            .enter()
            .append("rect")
            .attr("x", (d) => x(d.channel.toString()))
            .attr("y", (d) => y(d.occupancy))
            .attr("width", x.bandwidth())
            .attr("height", (d) => height - y(d.occupancy))
            .attr("fill", "#4f46e5");

        // Add labels
        svg.append("text")
            .attr("x", -height / 2)
            .attr("y", -margin.left + 20)
            .attr("transform", "rotate(-90)")
            .attr("text-anchor", "middle")
            .text("Occupancy");

        svg.append("text")
            .attr("x", width / 2)
            .attr("y", height + margin.bottom - 5)
            .attr("text-anchor", "middle")
            .text("Channel");
    };

    // Handle resize
    let resizeTimer: NodeJS.Timeout;
    const handleResize = () => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(createChart, 250);
    };

    onMount(() => {
        createChart();
        window.addEventListener("resize", handleResize);

        return () => {
            window.removeEventListener("resize", handleResize);
        };
    });

    $: if (normalizedData) {
        if (chartContainer) createChart();
    }
</script>

<div class="space-y-6">
    <div class="rounded-lg shadow p-6">
        <h2 class="text-xl font-bold mb-4">Channel Occupancy Overview</h2>
        <div bind:this={chartContainer} class="w-full h-[300px]"></div>
    </div>

    <div class="rounded-lg shadow">
        <div class="p-4 border-b">
            <div class="flex justify-between items-center">
                <h2 class="text-xl font-bold">Channel Details</h2>
                <select
                    class="rounded bg-gray-700 border p-2"
                    bind:value={sortBy}
                >
                    <option value="channel">Sort by Channel</option>
                    <option value="occupancy">Sort by Occupancy</option>
                </select>
            </div>
        </div>

        <div class="overflow-x-auto">
            <table class="w-full">
                <thead>
                    <tr class="bg-gray-50">
                        <th class="p-4 text-left">Channel</th>
                        <th class="p-4 text-left">Occupancy</th>
                        <th class="p-4 text-left">Quality</th>
                        <th class="p-4 text-left">Recommendation</th>
                    </tr>
                </thead>
                <tbody>
                    {#each sortedData as channel}
                        <tr class="border-t">
                            <td class="p-4 font-medium">{channel.channel}</td>
                            <td class="p-4"
                                >{formatOccupancy(channel.occupancy)}</td
                            >
                            <td
                                class="p-4 font-medium {getChannelColor(
                                    channel.occupancy,
                                )}"
                            >
                                {getChannelQuality(channel.occupancy)}
                            </td>
                            <td class="p-4">
                                {#if channel.occupancy < 0.3}
                                    Recommended, low traffic
                                {:else if channel.occupancy >= 0.7}
                                    Avoid - high congestion
                                {:else}
                                    Usable with caution
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    </div>
</div>

<style>
    table {
        @apply w-full border-separate border-2 border-gray-500 text-sm;
    }

    th {
        @apply p-2 text-center border-b border-gray-700 bg-gray-600;
    }

    td {
        @apply p-2 text-left text-lg font-semibold;
    }

    :global(svg) {
        width: 100%;
        height: 100%;
    }
</style>
