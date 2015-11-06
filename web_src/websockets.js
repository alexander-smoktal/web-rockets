var url = "localhost:8081";

var socket = new WebSocket("ws://" + url + "/");

socket.onopen = function () {
    document.getElementById("chat-window").value += "\nService: Connected to the server '" + url + "'";
};

socket.onclose = function () {
    document.getElementById("chat-window").value += "\nService: Disconnected from the server";
};

socket.onmessage = function (event) {
    document.getElementById("chat-window").value += "\nServer: " + event.data;
};

function send_message() {
    var message = document.getElementById("chat-message").value;
    document.getElementById("chat-message").value = "";

    socket.send(message);

    document.getElementById("chat-window").value += "\nMe: " + message;
}
