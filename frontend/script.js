function sendPostRequest(url, data) {
  console.log("Sending POST request to " + url);
  console.log(data);
  console.log(JSON.stringify(data));
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      "Content-Type": "application/json",
    },
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
  let dialog = document.getElementById("login-dialog");
  dialog.showModal();

  document.getElementById("login-button").onclick = async () => {
    dialog.close();
    try {
      const response = await sendPostRequest("/api/login", {
        username: document.getElementById("username").value,
        password: document.getElementById("password").value,
      });

      if (response.status === 200) {
        // Handle the successful response here
        window.location.reload();
      } else {
        const errorText = await response.text();
        Swal.fire({
          icon: "error",
          title: "Login failed",
          text: errorText,
        });
      }
    } catch (error) {
      Swal.fire({
        icon: "error",
        title: "Login failed",
        text: "Something went wrong",
      });
    }
  }
}

document.addEventListener("DOMContentLoaded", function () {
  var calendarEl = document.getElementById("calendar-div");
  calendar = new FullCalendar.Calendar(calendarEl, {
    initialView: "timeGridWeek",
    events: '/api/book/events',
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
  return new Promise((resolve, reject) => {
    // fill in the form with information of the resources
    let dropdown = document.getElementById("resources-dropdown");
    getResources().then((resources) => {
      resources.sort();
      resources.forEach((resource) => {
        let option = document.createElement("option");
        option.value = resource[0];
        option.innerHTML = resource[1];
        dropdown.appendChild(option);
      });
    })

    let start = document.getElementById("booking-start");
    let end = document.getElementById("booking-end");
    start.value = info.startStr.slice(0, -6);
    end.value = info.endStr.slice(0, -6);

    let dialog = document.getElementById("create-booking-dialog");
    dialog.showModal();
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
    dialog.addEventListener("close", (event) => {
      calendar.refetchEvents();
      calendar.unselect();
    });
    document.getElementById("create-booking-button").onclick = async () => {
      dialog.close();
      try {
        const response = await sendPostRequest("/api/book/new", {
          start_time: rfc3339(start.value),
          end_time: rfc3339(end.value),
          resource_name: dropdown.value,
        });

        if (response.status === 200) {
          // Handle the successful response here

          Toast.fire({
            icon: "success",
            title: "Booking successful"
          });
          resolve();
        } else {
          const errorText = await response.text();
          Toast.fire({
            icon: "error",
            title: "Booking failed: " + errorText
          });
          resolve();
        }
      } catch (error) {
        Toast.fire({
          icon: "error",
          title: "Booking failed"
        });
        resolve();
      }
    };
    document.getElementById("cancel-booking-button").onclick = () => {
      dialog.close();
      resolve();
    }
  });
}

async function getResources() {
  const response = await fetch('api/book/resources');
  const resources = await response.json();
  // console.log(resources);
  // return a list of resource name strings
  let resourceNames = [];
  for (const [key, value] of Object.entries(resources)) {
    // console.log(key, value);
    resourceNames.push([key, value.name]);
  }
  return resourceNames;
}

function rfc3339(d) {
  var d = new Date(d);
  function pad(n) {
    return n < 10 ? "0" + n : n;
  }

  function timezoneOffset(offset) {
    var sign;
    if (offset === 0) {
      return "Z";
    }
    sign = (offset > 0) ? "-" : "+";
    offset = Math.abs(offset);
    return sign + pad(Math.floor(offset / 60)) + ":" + pad(offset % 60);
  }

  return d.getFullYear() + "-" +
    pad(d.getMonth() + 1) + "-" +
    pad(d.getDate()) + "T" +
    pad(d.getHours()) + ":" +
    pad(d.getMinutes()) + ":" +
    pad(d.getSeconds()) +
    timezoneOffset(d.getTimezoneOffset());
}