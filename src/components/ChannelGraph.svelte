<script lang="ts">
    import { onMount } from "svelte";
    import * as d3 from "d3";

    export let channelData: { channel: number; occupancy: number }[];

    let svgElement: SVGSVGElement;

    // Define the dimensions and margins for the chart
    const margin = { top: 20, right: 30, bottom: 40, left: 40 };
    const width = 600 - margin.left - margin.right;
    const height = 400 - margin.top - margin.bottom;

    onMount(() => {
        // Sort the data by channel number before rendering the chart
        const sortedData = [...channelData].sort(
            (a, b) => a.channel - b.channel,
        );

        // Remove any existing chart before rendering the new one
        d3.select(svgElement).selectAll("*").remove();

        // Create the SVG container
        const svg = d3
            .select(svgElement)
            .attr("width", width + margin.left + margin.right)
            .attr("height", height + margin.top + margin.bottom)
            .append("g")
            .attr("transform", `translate(${margin.left},${margin.top})`);

        // Set the scales for the x and y axes
        const xScale = d3
            .scaleBand()
            .domain(sortedData.map((d) => d.channel.toString())) // Map channels as strings for the x-axis
            .range([0, width])
            .padding(0.1); // Add padding between bars

        const yScale = d3
            .scaleLinear()
            .domain([0, 1]) // Occupancy is between 0 and 1
            .range([height, 0]);

        // Add the X axis
        svg.append("g")
            .attr("transform", `translate(0, ${height})`)
            .call(d3.axisBottom(xScale));

        // Add the Y axis
        svg.append("g").call(d3.axisLeft(yScale));

        // Draw the bars
        svg.selectAll(".bar")
            .data(sortedData)
            .enter()
            .append("rect")
            .attr("class", "bar")
            .attr("x", (d) => xScale(d.channel.toString())!)
            .attr("y", (d) => yScale(d.occupancy))
            .attr("width", xScale.bandwidth())
            .attr("height", (d) => height - yScale(d.occupancy))
            .attr("fill", "#69b3a2");

        // Add labels above the bars
        svg.selectAll(".label")
            .data(sortedData)
            .enter()
            .append("text")
            .attr("class", "label")
            .attr(
                "x",
                (d) => xScale(d.channel.toString())! + xScale.bandwidth() / 2,
            )
            .attr("y", (d) => yScale(d.occupancy) - 5)
            .attr("text-anchor", "middle")
            .text((d) => `${(d.occupancy * 100).toFixed(1)}%`);
    });
</script>

<svg bind:this={svgElement}></svg>

<style>
    .bar {
        @apply fill-current text-teal-400; /* replace steelblue with teal color from Tailwind */
    }

    .label {
        @apply text-xs font-bold fill-white; /* font-size: 12px and fill: black */
    }
</style>
