// In explorer-ui/src/components/GraphCanvas.tsx

import React, { useEffect, useState, useRef } from 'react';
import axios from 'axios';
import * as d3 from 'd3';
import type { GraphData, GraphNode } from '../types'; // Import our new types
// Import our new types

// --- The D3 Visualization Logic ---
// We are putting the complex D3 code into its own separate function for cleanliness.
// It takes a reference to the SVG element and the data to draw.
const drawGraph = (svgElement: SVGSVGElement, data: GraphData) => {
    const svg = d3.select(svgElement);
    svg.selectAll("*").remove(); // Clear previous render

    const width = 800;
    const height = 600;

    // The "force simulation" is the physics engine that positions our nodes.
    const simulation = d3.forceSimulation(data.nodes as d3.SimulationNodeDatum[])
        .force("link", d3.forceLink(data.edges).id(d => (d as GraphNode).id).distance(100))
        .force("charge", d3.forceManyBody().strength(-200))
        .force("center", d3.forceCenter(width / 2, height / 2));

    // Draw the EDGES (links) first, so they are underneath the nodes.
    const link = svg.append("g")
        .selectAll("line")
        .data(data.edges)
        .join("line")
        .attr("stroke", "#999")
        .attr("stroke-opacity", 0.6);

    // Draw the NODES
    const node = svg.append("g")
        .selectAll("circle")
        .data(data.nodes)
        .join("circle")
        .attr("r", 10)
        .attr("fill", "#69b3a2")
        .call(drag(simulation) as any); // Make nodes draggable

    // Add labels to the nodes
    const label = svg.append("g")
        .selectAll("text")
        .data(data.nodes)
        .join("text")
        .text(d => d.label)
        .attr("x", 8)
        .attr("y", 3)
        .style("font-size", "12px")
        .style("fill", "#555");

    // The 'tick' function runs for every step of the physics simulation.
    // It updates the position of the nodes and links on the screen.
    simulation.on("tick", () => {
        link
            .attr("x1", d => (d.source as any).x)
            .attr("y1", d => (d.source as any).y)
            .attr("x2", d => (d.target as any).x)
            .attr("y2", d => (d.target as any).y);

        node
            .attr("cx", d => (d as any).x)
            .attr("cy", d => (d as any).y);

        label
            .attr("x", d => (d as any).x + 12)
            .attr("y", d => (d as any).y + 4);
    });

    // Helper function for dragging nodes
    function drag(simulation: d3.Simulation<d3.SimulationNodeDatum, undefined>) {
        // ... (this is standard D3 boilerplate for dragging)
        function dragstarted(event: any) {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            event.subject.fx = event.subject.x;
            event.subject.fy = event.subject.y;
        }
        function dragged(event: any) {
            event.subject.fx = event.x;
            event.subject.fy = event.y;
        }
        function dragended(event: any) {
            if (!event.active) simulation.alphaTarget(0);
            event.subject.fx = null;
            event.subject.fy = null;
        }
        return d3.drag()
            .on("start", dragstarted)
            .on("drag", dragged)
            .on("end", dragended);
    }
};


const GraphCanvas = () => {
    const svgRef = useRef<SVGSVGElement | null>(null);

    // 1. Create a state variable to hold our graph data.
    // It starts as an empty graph.
    const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });

    // 2. Create a state variable for loading/error states.
    const [status, setStatus] = useState<string>('Loading...');

    // 3. This useEffect hook will run ONCE when the component first mounts.
    useEffect(() => {
        const fetchGraphData = async () => {
            try {
                // IMPORTANT: Your Rust server from `cargo run --bin mre` MUST be running!
                // We make the API call to our backend.
                const backendUrl = import.meta.env.VITE_BACKEND_URL || 'http://localhost:8080';
                const response = await axios.get<GraphData>(`${backendUrl}/graph`);

                // If successful, update our component's state with the new data.
                setGraphData(response.data);
                setStatus('Data loaded successfully.');
            } catch (error) {
                console.error("Failed to fetch graph data:", error);
                setStatus('Failed to load data. Is the backend server running?');
            }
        };

        fetchGraphData();
    }, []); // The empty array `[]` means "run this effect only once."

    // 4. This useEffect hook will run WHENEVER `graphData` changes.
    useEffect(() => {
        // If we have a valid SVG element and some data, call our drawing function.
        if (svgRef.current && graphData.nodes.length > 0) {
            drawGraph(svgRef.current, graphData);
        }
    }, [graphData]); // The `[graphData]` means "run this effect when graphData changes."


    return (
        <div>
            <h2>Graph Visualization</h2>
            <p>Status: {status}</p>
            <svg ref={svgRef} width="800" height="600" style={{ border: '1px solid black' }}></svg>
        </div>
    );
};

export default GraphCanvas;