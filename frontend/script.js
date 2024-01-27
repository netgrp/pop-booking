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
    name: "John Doe",
    email: "John@Doe.com",
    room: 42,
    resource_name: form.resource_name.value,
    start_time: form.start_time.value,
    end_time: form.end_time.value,
  };
  console.log(data);
  sendPostRequest("/new", data)
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
    selectable: true,
    selectMirror: true,
    unselectAuto: false,
    weekNumbers: true,
    select: calendarSelect,
    headerToolbar: {
      left: 'today',
      center: 'title',
      right: 'prev,next' // user can switch between the two
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
      }
    },
    firstDay: 1,
    locale: "dk",
  });
  calendar.render();
});

async function calendarSelect(info) {
  const { value: formValues } = await Swal.fire({
    title: "Booking",
    html: `
      <select id="swal-select" class="swal2-select">
        <option value="option1">Option 1</option>
        <option value="option2">Option 2</option>
        <option value="option3">Option 3</option>
      </select>
    `,
    focusConfirm: false,
    preConfirm: () => {
      return document.getElementById("swal-select").value;
    }
  });
  if (formValues) {
    const Toast = Swal.mixin({
      toast: true,
      position: "bottom-end",
      showConfirmButton: false,
      timer: 3000,
      timerProgressBar: true,
      didOpen: (toast) => {
        toast.onmouseenter = Swal.stopTimer;
        toast.onmouseleave = Swal.resumeTimer;
      }
    });
    Toast.fire({
      icon: "success",
      title: "Booking succesful"
    });
  }
  calendar.unselect()
}