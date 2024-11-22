const charts = {};
const load_time = new Date();

document.addEventListener("DOMContentLoaded", () => {
    const messageCountMax = document.getElementById('messageCountMax').getAttribute("value");
    // paint charts
    paintTasksChart();
    paintCpuChart();
    paintMemChart();
    // Start Websocket
    let ws_protocol = "wss://";
    if (window.location.protocol === 'http:') {
        ws_protocol = "ws://";
    }
    const socket = new WebSocket(ws_protocol + window.location.host + '/ws');
    socket.onmessage = onWsMessage;
});

function onWsMessage(event) {
    var message = JSON.parse(event.data);
    let time = new Date(message.time);

    document.getElementById('redis_keys_stats').innerHTML = message["keys"];
    document.getElementById('last_update_stats').innerHTML = time.toLocaleString();
    //console.log(event);

    // update charts
    updateTasksChart(message, time);
    updateCpuChart(message, time);
    updateMemChart(message, time);
}

function updateMemChart(message, time) {
    // update charts
    if (charts['mem_chart'].data.labels.length > messageCountMax) {
        charts['mem_chart'].data.labels.shift();
        charts['mem_chart'].data.datasets[0].data.shift();
        charts['mem_chart'].data.datasets[1].data.shift();
    }
    charts['mem_chart'].data.labels.push(time);
    charts['mem_chart'].data.datasets[0].data.push(message.memory_sys);
    charts['mem_chart'].data.datasets[1].data.push(message.memory);
    charts['mem_chart'].update();
}

function updateCpuChart(message, time) {
    // update charts
    if (charts['cpu_chart'].data.labels.length > messageCountMax) {
        charts['cpu_chart'].data.labels.shift();
        charts['cpu_chart'].data.datasets[0].data.shift();
        charts['cpu_chart'].data.datasets[1].data.shift();
    }
    charts['cpu_chart'].data.labels.push(time);
    charts['cpu_chart'].data.datasets[0].data.push(message.cpu);
    charts['cpu_chart'].data.datasets[1].data.push(message.cpu_proc);
    charts['cpu_chart'].update();
}

function updateTasksChart(message, time) {
    // update charts
    if (charts['tasks_chart'].data.labels.length > messageCountMax) {
        charts['tasks_chart'].data.labels.shift();
        charts['tasks_chart'].data.datasets[0].data.shift();
        charts['tasks_chart'].data.datasets[1].data.shift();
    }
    charts['tasks_chart'].data.labels.push(time);
    charts['tasks_chart'].data.datasets[0].data.push(message.tasks);
    charts['tasks_chart'].data.datasets[1].data.push(message.sync_threads);
    charts['tasks_chart'].update();
}

function paintCpuChart() {
    const ctx = document.getElementById('cpu_chart').getContext('2d');
    const cpu_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: [
            ],
            datasets: [{
                label: "CPU System",
                data: [],
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            }, {
                label: "Cpu Process",
                data: [],
                backgroundColor: '#0d6efd',
                borderColor: '#0d6efd',
                borderWidth: 1
            }]
        },
        options: {
            elements: { point: { radius: 0 } },
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
                    display: true,
                    title: {
                        display: true,
                        text: '%'
                    },
                    min: 0,
                    max: 100
                },
                x: {
                    type: 'timeseries',
                    display: false
                }
            }
        }
    });
    charts['cpu_chart'] = cpu_chart;
}
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
            }, {
                label: "Sync Threads",
                data: [],
                backgroundColor: '#0d6efd',
                borderColor: '#0d6efd',
                borderWidth: 1
            }]
        },
        options: {
            elements: { point: { radius: 0 } },
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
                    display: true,
                    title: {
                        display: true,
                        text: '#'
                    },
                },
                x: {
                    type: 'timeseries',
                    display: false
                }
            }
        }
    });
    charts['tasks_chart'] = tasks_chart;
}

function paintMemChart() {
    const ctx = document.getElementById('mem_chart').getContext('2d');
    const mem_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: [
            ],
            datasets: [{
                label: "System Memory",
                data: [],
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            }, {
                label: "Process Memory",
                data: [],
                backgroundColor: '#0d6efd',
                borderColor: '#0d6efd',
                borderWidth: 1
            }]
        },
        options: {
            elements: { point: { radius: 0 } },
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
                    display: true,
                    title: {
                        display: true,
                        text: 'MB'
                    },
                },
                x: {
                    type: 'timeseries',
                    display: false
                }
            }
        }
    });
    charts['mem_chart'] = mem_chart;
}