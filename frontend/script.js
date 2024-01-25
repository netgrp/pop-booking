function sendPostRequest(url, data) {
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      "Content-Type": "application/json",
    },
  });
}

function updateForm() {
  const form = document.getElementById("form");
  const data = {
    name: form.name.value,
    email: form.email.value,
    message: form.message.value,
  };
  console.log(data);
  sendPostRequest("/new_booking", data)
    .then((response) => {
      if (response.status === 200) {
        alert("Message sent!");
      } else {
        alert("Something went wrong...");
      }
    })
    .catch((error) => {
      console.error("Error:", error);
    });
}

function switchPages(toPage) {
  let messageDiv = document.getElementById("notices-div");
  let calendarDiv = document.getElementById("calendar-div");
  let messageButton = document.getElementById("notices");
  let calendarButton = document.getElementById("calendar");

  switch (toPage) {
    case "notices":
      messageDiv.removeAttribute("hidden");
      messageButton.setAttribute("class", "active");
      calendarDiv.setAttribute("hidden", "");
      calendarButton.setAttribute("class", "");
      break;
    case "calendar":
      messageDiv.setAttribute("hidden", "");
      messageButton.setAttribute("class", "");
      calendarDiv.removeAttribute("hidden");
      calendarButton.setAttribute("class", "active");
      break;
  }
}

function showLoginForm() {
  Swal.fire({
    title: 'Login Form',
    html: `
      <input type="text" id="username" class="swal2-input" placeholder="Username">
      <input type="password" id="password" class="swal2-input" placeholder="Password">
    `,
    confirmButtonText: 'Sign in',
    focusConfirm: false,
    didOpen: () => {
      const popup = Swal.getPopup()
      usernameInput = popup.querySelector('#username')
      passwordInput = popup.querySelector('#password')
      usernameInput.onkeyup = (event) => event.key === 'Enter' && Swal.clickConfirm()
      passwordInput.onkeyup = (event) => event.key === 'Enter' && Swal.clickConfirm()
    },
    preConfirm: () => {
      const username = usernameInput.value
      const password = passwordInput.value
      if (!username || !password) {
        Swal.showValidationMessage(`Please enter username and password`)
      }
      return { username, password }
    },
  })
}

document.addEventListener("DOMContentLoaded", function () {
  var calendarEl = document.getElementById("calendar-div");
  calendar = new FullCalendar.Calendar(calendarEl, {
    initialView: "timeGridWeek",
    height: "100%",
    headerToolbar: {
      left: 'prev,next',
      center: 'title',
      right: 'timeGridWeek,timeGridDay' // user can switch between the two
    },
    views: {
      timeGridWeek: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        slotLabelInterval: '01:00',
        buttonText: 'Week',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          week: 'numeric',
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridDay: {
        type: 'timeGrid',
        duration: { days: 1 },
        buttonText: 'Day'
      }
    },
    firstDay: 1,
    locale: "dk",
  });
  calendar.render();
});