const charts = {};
const load_time = new Date();

document.addEventListener("DOMContentLoaded", () => {
    const messageCountMax = document.getElementById('messageCountMax').getAttribute("value");
    let statsHistory = JSON.parse(document.getElementById('statsHistory').getAttribute("value"));
    // paint charts
    paintTasksChart(statsHistory);
    paintCpuChart(statsHistory);
    paintMemChart(statsHistory);
    // Start Websocket
    let ws_protocol = "wss://";
    if (window.location.protocol === 'http:') {
        ws_protocol = "ws://";
    }
    const statusSocket = new WebSocket(ws_protocol + window.location.host + '/stats/ws');
    statusSocket.onmessage = onStatusMessage;
    loadSoccerField();
});

function onStatusMessage(event) {
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
    charts['mem_chart'].data.datasets[0].data.push(message.mem);
    charts['mem_chart'].data.datasets[1].data.push(message.mem_proc);
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

function paintCpuChart(statsHistory) {
    const ctx = document.getElementById('cpu_chart').getContext('2d');
    let data_cpu = statsHistory.map(val => val.cpu);
    let data_cpu_proc = statsHistory.map(val => val.cpu_proc);
    let data_timestamps = statsHistory.map(val => new Date(val.time));
    const cpu_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: data_timestamps,
            datasets: [{
                label: "CPU System",
                data: data_cpu,
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            }, {
                label: "Cpu Process",
                data: data_cpu_proc,
                backgroundColor: '#0d6efd',
                borderColor: '#0d6efd',
                borderWidth: 1
            }]
        },
        options: {
            elements: { point: { radius: 0 } },
            // animations: {
            //     radius: {
            //         duration: 400,
            //         easing: 'linear',
            //         loop: (context) => context.active
            //     }
            // },
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
function paintTasksChart(statsHistory) {
    const ctx = document.getElementById('tasks_chart').getContext('2d');
    let data_tasks = statsHistory.map(val => val.tasks);
    let data_sync_tasks = statsHistory.map(val => val.sync_threads);
    let data_timestamps = statsHistory.map(val => new Date(val.time));
    const tasks_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: data_timestamps,
            datasets: [{
                label: "Tasks",
                data: data_tasks,
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            }, {
                label: "Sync Threads",
                data: data_sync_tasks,
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

function paintMemChart(statsHistory) {
    const ctx = document.getElementById('mem_chart').getContext('2d');
    let data_mem = statsHistory.map(val => val.mem);
    let data_mem_proc = statsHistory.map(val => val.mem_proc);
    let data_timestamps = statsHistory.map(val => new Date(val.time));
    const mem_chart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: data_timestamps,
            datasets: [{
                label: "System Memory",
                data: data_mem,
                backgroundColor: 'rgba(75, 192, 192, 0.2)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1
            }, {
                label: "Process Memory",
                data: data_mem_proc,
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

function loadSoccerField() {
    let field = document.getElementById('soccerField');
    let fieldRect = field.getBoundingClientRect();
    let fieldX = fieldRect.x;
    let fieldY = fieldRect.y;

    const ball = document.getElementById('ball');
    let ws_protocol = "wss://";
    if (window.location.protocol === 'http:') {
        ws_protocol = "ws://";
    }
    const socket = new WebSocket(ws_protocol + window.location.host + '/soccer_field/ws');
    socket.onmessage = function (event) {
        let doc = JSON.parse(event.data);
        ball.setAttribute('cy', doc.ball[0]);
        ball.setAttribute('cx', doc.ball[1]);
        for (let [key, value] of Object.entries(doc.players)) {
            let player = document.getElementById(`player_${key}`);
            if (player == undefined) {
                var newElement = document.createElementNS("http://www.w3.org/2000/svg", 'circle');
                newElement.setAttribute("r", 8);
                newElement.setAttribute("id", `player_${key}`);
                newElement.setAttribute("class", `player`);
                field.appendChild(newElement);
                player = newElement;
            }
            player.setAttribute('cy', value[0]);
            player.setAttribute('cx', value[1]);
        }

        // cleanup all circles
        let circles = field.getElementsByTagName('circle');
        for (let i = 0; i < circles.length; i++) {
            let id = circles[i].getAttribute("id");
            if (id == "ball" || id == null) {
                continue;
            }
            id = id.split("_")[1];
            if (!(id in doc.players)) {
                console.log(id);
                circles[i].remove();
            }
        }
    }

    let targetX = 0; // Target X position (mouse position)
    let targetY = 0; // Target Y position (mouse position)

    let old_targetX = 0;
    let old_targetY = 0;

    setInterval(() => {
        if(targetX != old_targetX || targetY != old_targetY) {
            socket.send(`[${targetY},${targetX}]`);
            old_targetX = targetX;
            old_targetY = targetY;
        }
    }, 50); // Update every 100 milliseconds

    // Update target position on mouse move
    document.addEventListener('mousemove', (event) => {
        let fieldRect = field.getBoundingClientRect();
        fieldX = fieldRect.x;
        fieldY = fieldRect.y;
        if((event.clientX >= fieldX && event.clientX < fieldX + 400)
        && (event.clientY >= fieldY && event.clientY < fieldY + 800)) {
            // if target is in rect
            targetX = event.clientX - fieldX;
            targetY = event.clientY - fieldY;
        }
    });
}