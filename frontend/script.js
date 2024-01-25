function sendPostRequest(url, data) {
    return fetch(url, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: {
            'Content-Type': 'application/json',
        },
    })
}

function updateForm() {
    const form = document.getElementById('form')
    const data = {
        name: form.name.value,
        email: form.email.value,
        message: form.message.value,
    }
    console.log(data)
    sendPostRequest('/new_booking', data)
        .then((response) => {
            if (response.status === 200) {
                alert('Message sent!')
            } else {
                alert('Something went wrong...')
            }
        })
        .catch((error) => {
            console.error('Error:', error)
        })
}