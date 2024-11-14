const charts = {};
const load_time = new Date();

var messageCount = 0;

document.addEventListener("DOMContentLoaded", () => {
    // paint charts
    paintTasksChart();
    // Start Websocket
    let ws_protocol = "wss://";
    if (window.location.protocol === 'http:') {
        ws_protocol = "ws://";
    }
    const socket = new WebSocket(ws_protocol + window.location.host + '/ws');
    socket.onmessage = (event) => {
        var message = JSON.parse(event.data);
        let time = new Date(message.time);

        document.getElementById('cpu_stats').innerHTML = Math.round(message["cpu"] * 1000) / 1000;
        document.getElementById('memory_stats').innerHTML = message["memory"];
        document.getElementById('redis_keys_stats').innerHTML = message["keys"];
        document.getElementById('tasks_stats').innerHTML = message["tasks"];
        document.getElementById('sync_threads_stats').innerHTML = message["sync_threads"];
        document.getElementById('last_update_stats').innerHTML = time.toLocaleString();
        //console.log(event);
        
        // update charts
        if (messageCount > 60) {
            charts['tasks_chart'].data.labels.shift();
            charts['tasks_chart'].data.datasets[0].data.shift();
            charts['tasks_chart'].data.datasets[1].data.shift();
        }
        charts['tasks_chart'].data.labels.push(time);
        charts['tasks_chart'].data.datasets[0].data.push(message.tasks);
        charts['tasks_chart'].data.datasets[1].data.push(message.sync_threads);
        charts['tasks_chart'].update();
        messageCount += 1;
    }
});

function paintTasksChart() {
    const ctx = document.getElementById('tasks_chart').getContext('2d');
    const tasks_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: [
            ],
            datasets: [{
                label: "Tasks",
                data: [],
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            },{
                label: "Sync Threads",
                data: [],
                backgroundColor: '#0d6efd',
                borderColor: '#0d6efd',
                borderWidth: 1
            }]
        },
        options: {
            animations: {
                radius: {
                  duration: 400,
                  easing: 'linear',
                  loop: (context) => context.active
                }
              },
            scales: {
                y: {
                    beginAtZero: true,
                },
                x: {
                    type: 'timeseries',
                }
            }
        }
    });
    charts['tasks_chart'] = tasks_chart;
}